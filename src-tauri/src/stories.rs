// stories.rs — Persistent story registry
//
// Stores story metadata in SQLite (`stories` table).
// Manuscript and report documents remain filesystem-based under story folders.

use rusqlite::{params, Connection};
use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri::Manager;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Story {
    pub id:      String,
    pub name:    String,
    pub folder:  String,
    pub created: String,
    #[serde(default)]
    pub bible_path: String,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn load_stories(conn: &Connection) -> Result<Vec<Story>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, folder, created, bible_path
             FROM stories
             ORDER BY created DESC, name COLLATE NOCASE ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |r| {
            Ok(Story {
                id: r.get(0)?,
                name: r.get(1)?,
                folder: r.get(2)?,
                created: r.get(3)?,
                bible_path: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn new_id() -> String {
    // Simple timestamp-based ID — no uuid crate needed
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{:x}", ts)
}

// ── Commands ──────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct StoriesResult {
    pub success: bool,
    pub stories: Vec<Story>,
    pub error:   String,
}

#[derive(Deserialize)]
pub struct InitStoryRequest {
    pub name:          String,
    pub parent_folder: String,
}

#[derive(Deserialize)]
pub struct AddStoryRequest {
    pub name:   String,
    pub folder: String,
}

#[derive(Deserialize)]
pub struct UpdateStoryRequest {
    pub id:     String,
    pub name:   String,
    pub folder: String,
    #[serde(default)]
    pub bible_path: String,
}

/// Turn a story name into a safe single path segment.
fn sanitize_folder_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c if c.is_control() => '-',
            c => c,
        })
        .collect::<String>()
        .trim()
        .trim_matches('.')
        .to_string();
    if cleaned.is_empty() {
        "Untitled Story".to_string()
    } else {
        cleaned
    }
}

fn ensure_story_scaffold(story_dir: &std::path::Path) -> Result<(), String> {
    let structure = crate::folder_structure::current();
    for sub in structure.scaffold_dirs() {
        fs::create_dir_all(story_dir.join(&sub))
            .map_err(|e| format!("Cannot create {}/: {}", sub, e))?;
    }
    Ok(())
}

/// List all stories.
#[tauri::command]
pub async fn list_stories(app: AppHandle) -> StoriesResult {
    let db = app.state::<crate::db::Db>();
    let conn = match db.0.lock() {
        Ok(conn) => conn,
        Err(e) => {
            return StoriesResult { success: false, stories: Vec::new(), error: e.to_string() };
        }
    };

    match load_stories(&conn) {
        Ok(stories) => StoriesResult { success: true, stories, error: String::new() },
        Err(e)      => StoriesResult { success: false, stories: Vec::new(), error: e },
    }
}

fn register_story(db: &crate::db::Db, name: String, folder: String) -> StoriesResult {
    let now = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let story = Story {
        id:         new_id(),
        name,
        folder,
        created:    now,
        bible_path: String::new(),
    };

    let conn = match db.0.lock() {
        Ok(conn) => conn,
        Err(e) => return StoriesResult { success: false, stories: Vec::new(), error: e.to_string() },
    };

    if let Err(e) = conn.execute(
        "INSERT INTO stories (id, name, folder, created, bible_path) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![story.id, story.name, story.folder, story.created, story.bible_path],
    ) {
        let msg = e.to_string();
        if msg.contains("UNIQUE constraint failed: stories.folder") {
            return StoriesResult {
                success: false,
                stories: Vec::new(),
                error: "This story folder is already registered.".to_string(),
            };
        }
        return StoriesResult { success: false, stories: Vec::new(), error: msg };
    }

    match load_stories(&conn) {
        Ok(stories) => StoriesResult { success: true, stories, error: String::new() },
        Err(e) => StoriesResult { success: false, stories: Vec::new(), error: e },
    }
}

/// Register an existing story folder (does not create folders).
#[tauri::command]
pub async fn add_story(app: AppHandle, request: AddStoryRequest) -> StoriesResult {
    let folder = PathBuf::from(&request.folder);
    if !folder.is_dir() {
        return StoriesResult {
            success: false, stories: Vec::new(),
            error: format!("Folder does not exist: {}", request.folder),
        };
    }
    let db = app.state::<crate::db::Db>();
    register_story(&db, request.name.trim().to_string(), request.folder.clone())
}

/// Create a new empty story folder named after the story, with the configured
/// subfolders from Settings → Folder Structure, then register it.
#[tauri::command]
pub async fn init_story(app: AppHandle, request: InitStoryRequest) -> StoriesResult {
    // Ensure cache matches disk before scaffolding
    let _ = crate::folder_structure::load(&app);
    let name = request.name.trim().to_string();
    if name.is_empty() {
        return StoriesResult {
            success: false, stories: Vec::new(),
            error: "Please enter a story name.".to_string(),
        };
    }

    let parent = PathBuf::from(&request.parent_folder);
    if !parent.is_dir() {
        return StoriesResult {
            success: false, stories: Vec::new(),
            error: format!("Parent folder does not exist: {}", request.parent_folder),
        };
    }

    let story_dir = parent.join(sanitize_folder_name(&name));
    if story_dir.exists() {
        return StoriesResult {
            success: false, stories: Vec::new(),
            error: format!(
                "A folder already exists at: {}",
                story_dir.to_string_lossy()
            ),
        };
    }

    if let Err(e) = ensure_story_scaffold(&story_dir) {
        // Best-effort cleanup if we partially created the tree
        let _ = fs::remove_dir_all(&story_dir);
        return StoriesResult { success: false, stories: Vec::new(), error: e };
    }

    let db = app.state::<crate::db::Db>();
    register_story(&db, name, story_dir.to_string_lossy().to_string())
}

/// Update a story's name and/or folder.
#[tauri::command]
pub async fn update_story(app: AppHandle, request: UpdateStoryRequest) -> StoriesResult {
    let folder = PathBuf::from(&request.folder);
    if !folder.exists() {
        return StoriesResult {
            success: false, stories: Vec::new(),
            error: format!("Folder does not exist: {}", request.folder),
        };
    }

    let db = app.state::<crate::db::Db>();
    let conn = match db.0.lock() {
        Ok(conn) => conn,
        Err(e) => return StoriesResult { success: false, stories: Vec::new(), error: e.to_string() },
    };

    if let Err(e) = conn.execute(
        "UPDATE stories
         SET name = ?1, folder = ?2, bible_path = ?3
         WHERE id = ?4",
        params![request.name.trim(), request.folder, request.bible_path, request.id],
    ) {
        return StoriesResult { success: false, stories: Vec::new(), error: e.to_string() };
    }

    match load_stories(&conn) {
        Ok(stories) => StoriesResult { success: true, stories, error: String::new() },
        Err(e) => StoriesResult { success: false, stories: Vec::new(), error: e },
    }
}

/// Delete a story by id (does NOT delete the folder).
#[tauri::command]
pub async fn delete_story(app: AppHandle, id: String) -> StoriesResult {
    let db = app.state::<crate::db::Db>();
    let conn = match db.0.lock() {
        Ok(conn) => conn,
        Err(e) => return StoriesResult { success: false, stories: Vec::new(), error: e.to_string() },
    };

    if let Err(e) = conn.execute("DELETE FROM stories WHERE id = ?1", params![id]) {
        return StoriesResult { success: false, stories: Vec::new(), error: e.to_string() };
    }

    match load_stories(&conn) {
        Ok(stories) => StoriesResult { success: true, stories, error: String::new() },
        Err(e) => StoriesResult { success: false, stories: Vec::new(), error: e },
    }
}

#[derive(Deserialize)]
pub struct CreateDocumentRequest {
    pub story_folder: String,
    /// Display / title name (becomes the `.md` filename)
    pub name: String,
    /// Relative directory under the story folder (e.g. `Manuscript`, `Research`)
    pub location: String,
}

#[derive(Serialize)]
pub struct CreateDocumentResult {
    pub path: String,
    pub title: String,
}

/// Sanitize a document title into a safe `.md` filename stem.
fn sanitize_file_stem(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c if c.is_control() => '-',
            c => c,
        })
        .collect::<String>()
        .trim()
        .trim_matches('.')
        .to_string();
    if cleaned.is_empty() {
        "Untitled".to_string()
    } else {
        cleaned
    }
}

fn sanitize_relative_location(location: &str) -> Result<String, String> {
    let trimmed = location.trim().trim_matches('/').trim_matches('\\');
    if trimmed.is_empty() {
        return Err("Please choose a location for the new file.".to_string());
    }
    if trimmed.contains("..") {
        return Err("Location cannot contain '..'.".to_string());
    }
    Ok(trimmed.replace('\\', "/"))
}

/// Create a new empty markdown document under a story folder.
#[tauri::command]
pub async fn create_story_document(
    request: CreateDocumentRequest,
) -> Result<CreateDocumentResult, String> {
    let title = request.name.trim().to_string();
    if title.is_empty() {
        return Err("Please enter a document name.".to_string());
    }

    let story = PathBuf::from(&request.story_folder);
    if !story.is_dir() {
        return Err(format!("Story folder does not exist: {}", request.story_folder));
    }

    let location = sanitize_relative_location(&request.location)?;
    let dir = story.join(&location);
    fs::create_dir_all(&dir)
        .map_err(|e| format!("Cannot create {}: {}", dir.display(), e))?;

    let stem = sanitize_file_stem(&title);
    let file_path = dir.join(format!("{}.md", stem));
    if file_path.exists() {
        return Err(format!(
            "A file already exists at: {}",
            file_path.to_string_lossy()
        ));
    }

    let content = format!("# {}\n\n", title);
    fs::write(&file_path, content)
        .map_err(|e| format!("Cannot write {}: {}", file_path.display(), e))?;

    Ok(CreateDocumentResult {
        path: file_path.to_string_lossy().to_string(),
        title,
    })
}

/// Delete a document file. Path must resolve under the given story folder.
#[tauri::command]
pub async fn delete_story_document(
    story_folder: String,
    file_path: String,
) -> Result<(), String> {
    let story = PathBuf::from(&story_folder)
        .canonicalize()
        .map_err(|e| format!("Story folder not found: {}", e))?;
    let path = PathBuf::from(&file_path);
    let real = if path.exists() {
        path.canonicalize()
            .map_err(|e| format!("Cannot resolve {}: {}", file_path, e))?
    } else {
        return Err(format!("File does not exist: {}", file_path));
    };

    if !real.starts_with(&story) {
        return Err("Refusing to delete a file outside the story folder.".to_string());
    }
    if real.extension().and_then(|e| e.to_str()) != Some("md") {
        return Err("Only markdown (.md) documents can be deleted here.".to_string());
    }

    fs::remove_file(&real).map_err(|e| format!("Cannot delete {}: {}", real.display(), e))?;
    Ok(())
}

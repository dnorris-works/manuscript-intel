// series.rs — Series management (groups stories into reading order)

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use crate::db;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Series {
    pub id: i64,
    pub name: String,
    pub created_at: String,
    pub bible_path: String,
    pub books: Vec<SeriesBook>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SeriesBook {
    pub story_folder: String,
    pub story_name: String,
    pub book_order: i64,
}

#[derive(Serialize)]
pub struct SeriesResult {
    pub success: bool,
    pub series: Vec<Series>,
    pub error: String,
}

#[derive(Deserialize)]
pub struct CreateSeriesRequest {
    pub name: String,
    pub books: Vec<SeriesBookInput>,
    #[serde(default)]
    pub bible_path: String,
    #[serde(default)]
    pub create_empty: bool,
    #[serde(default)]
    pub parent_folder: String,
}

#[derive(Deserialize)]
pub struct UpdateSeriesRequest {
    pub id: i64,
    pub name: String,
    pub books: Vec<SeriesBookInput>,
    #[serde(default)]
    pub bible_path: String,
}

#[derive(Deserialize, Clone)]
pub struct SeriesBookInput {
    pub story_folder: String,
    pub story_name: String,
    pub book_order: i64,
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_series(app: AppHandle) -> SeriesResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();

    let mut stmt = match conn.prepare("SELECT id, name, created_at, COALESCE(bible_path, '') FROM series ORDER BY name") {
        Ok(s) => s,
        Err(e) => return SeriesResult { success: false, series: Vec::new(), error: e.to_string() },
    };

    let series_rows: Vec<(i64, String, String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
        .unwrap_or_default();

    let mut series_list = Vec::new();
    for (id, name, created_at, bible_path) in series_rows {
        let books = load_series_books(&conn, id);
        series_list.push(Series { id, name, created_at, bible_path, books });
    }

    SeriesResult { success: true, series: series_list, error: String::new() }
}

#[tauri::command]
pub async fn create_series(app: AppHandle, request: CreateSeriesRequest) -> SeriesResult {
    let name = request.name.trim().to_string();
    if name.is_empty() {
        return SeriesResult { success: false, series: Vec::new(), error: "Series name is required.".to_string() };
    }

    let mut bible_path = request.bible_path.trim().to_string();

    if request.create_empty {
        let _ = crate::folder_structure::load(&app);
        let parent = PathBuf::from(request.parent_folder.trim());
        if !parent.is_dir() {
            return SeriesResult {
                success: false,
                series: Vec::new(),
                error: format!("Parent folder does not exist: {}", request.parent_folder),
            };
        }

        let series_dir = parent.join(sanitize_folder_name(&name));
        if series_dir.exists() {
            return SeriesResult {
                success: false,
                series: Vec::new(),
                error: format!("A folder already exists at: {}", series_dir.to_string_lossy()),
            };
        }

        match ensure_series_scaffold(&series_dir) {
            Ok(default_bible_path) => {
                if bible_path.is_empty() {
                    bible_path = default_bible_path;
                }
            }
            Err(e) => {
                let _ = fs::remove_dir_all(&series_dir);
                return SeriesResult { success: false, series: Vec::new(), error: e };
            }
        }
    }

    {
        let database = app.state::<db::Db>();
        let conn = database.0.lock().unwrap();

        let now = chrono::Utc::now().to_rfc3339();
        if let Err(e) = conn.execute(
            "INSERT INTO series (name, created_at, bible_path) VALUES (?1, ?2, ?3)",
            rusqlite::params![name, now, bible_path],
        ) {
            return SeriesResult { success: false, series: Vec::new(), error: format!("Could not create series: {}", e) };
        }

        let series_id = conn.last_insert_rowid();
        save_series_books(&conn, series_id, &request.books);
    }
    list_series(app).await
}

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
        "Untitled Series".to_string()
    } else {
        cleaned
    }
}

fn ensure_series_scaffold(series_dir: &Path) -> Result<String, String> {
    let structure = crate::folder_structure::current();

    let mut dirs = vec![
        structure.bible().to_string(),
        structure.characters().to_string(),
        structure.locations().to_string(),
        "Books".to_string(),
    ];
    for extra in &structure.extra {
        let trimmed = extra.trim();
        if !trimmed.is_empty() {
            dirs.push(trimmed.to_string());
        }
    }

    for sub in dirs {
        fs::create_dir_all(series_dir.join(&sub))
            .map_err(|e| format!("Cannot create {}/: {}", sub, e))?;
    }

    let bible_file = series_dir.join(structure.bible()).join("Series-Bible.md");
    if !bible_file.exists() {
        fs::write(&bible_file, "# Series Bible\n\n")
            .map_err(|e| format!("Cannot write {}: {}", bible_file.display(), e))?;
    }

    Ok(bible_file.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn update_series(app: AppHandle, request: UpdateSeriesRequest) -> SeriesResult {
    {
        let database = app.state::<db::Db>();
        let conn = database.0.lock().unwrap();

        let name = request.name.trim();
        if name.is_empty() {
            return SeriesResult { success: false, series: Vec::new(), error: "Series name is required.".to_string() };
        }

        if let Err(e) = conn.execute(
            "UPDATE series SET name = ?1, bible_path = ?2 WHERE id = ?3",
            rusqlite::params![name, request.bible_path, request.id],
        ) {
            return SeriesResult { success: false, series: Vec::new(), error: format!("Could not update series: {}", e) };
        }

        save_series_books(&conn, request.id, &request.books);
    }
    list_series(app).await
}

#[tauri::command]
pub async fn delete_series(app: AppHandle, id: i64) -> SeriesResult {
    {
        let database = app.state::<db::Db>();
        let conn = database.0.lock().unwrap();
        let _ = conn.execute("DELETE FROM series_books WHERE series_id = ?1", rusqlite::params![id]);
        let _ = conn.execute("DELETE FROM series WHERE id = ?1", rusqlite::params![id]);
    }
    list_series(app).await
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn load_series_books(conn: &rusqlite::Connection, series_id: i64) -> Vec<SeriesBook> {
    let mut stmt = match conn.prepare(
        "SELECT story_folder, story_name, book_order FROM series_books WHERE series_id = ?1 ORDER BY book_order"
    ) { Ok(s) => s, Err(_) => return Vec::new() };

    stmt.query_map(rusqlite::params![series_id], |r| {
        Ok(SeriesBook {
            story_folder: r.get(0)?,
            story_name: r.get(1)?,
            book_order: r.get(2)?,
        })
    }).and_then(|rows| rows.collect::<Result<Vec<_>, _>>()).unwrap_or_default()
}

fn save_series_books(conn: &rusqlite::Connection, series_id: i64, books: &[SeriesBookInput]) {
    let _ = conn.execute("DELETE FROM series_books WHERE series_id = ?1", rusqlite::params![series_id]);
    for book in books {
        let _ = conn.execute(
            "INSERT INTO series_books (series_id, story_folder, story_name, book_order) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![series_id, book.story_folder, book.story_name, book.book_order],
        );
    }
}

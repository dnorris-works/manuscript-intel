// prompts.rs — Prompt system: load templates from DB, preprocess text, fill placeholders, call LLM.
//
// The prompt pipeline:
//   1. Load template from prompt_templates table by id
//   2. Load/generate preprocessed chapter text (cached in preprocessed_chapters)
//   3. Load bible text (if bible_path is set)
//   4. Fill placeholders in the user_template
//   5. Call LLM with system_prompt + filled user_template

use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::Path;

use crate::db::Db;

// ── Template loading ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PromptTemplate {
    pub system_prompt: String,
    pub user_template: String,
    pub max_tokens:    u32,
    pub json_mode:     bool,
}

/// Load a prompt template from the database by id.
pub fn load_template(conn: &Connection, template_id: &str) -> Result<PromptTemplate, String> {
    conn.query_row(
        "SELECT system_prompt, user_template, max_tokens, json_mode FROM prompt_templates WHERE id = ?1",
        params![template_id],
        |r| Ok(PromptTemplate {
            system_prompt: r.get(0)?,
            user_template: r.get(1)?,
            max_tokens:    r.get::<_, i64>(2)? as u32,
            json_mode:     r.get::<_, i64>(3)? != 0,
        }),
    ).map_err(|e| format!("Prompt template '{}' not found: {}", template_id, e))
}

// ── Bible loading ─────────────────────────────────────────────────────────────

/// How much story bible context to attach to a prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BibleTier {
    /// Full bible + characters + locations (up to 8k words).
    Full,
    /// Characters + locations only (up to 2k words) — summaries, continuity.
    Medium,
    /// No bible — mechanical checks (pacing, cliffhanger, SDT).
    Minimal,
}

const BIBLE_FULL_WORD_LIMIT: usize = 8000;
const BIBLE_MEDIUM_WORD_LIMIT: usize = 2000;

/// Load bible text for a story at the requested detail tier.
pub fn load_bible_tiered(story_folder: &str, explicit_bible_path: &str, tier: BibleTier) -> String {
    match tier {
        BibleTier::Minimal => String::new(),
        BibleTier::Medium => discover_bible_medium(story_folder),
        BibleTier::Full => load_bible_for_story(story_folder, explicit_bible_path),
    }
}

fn discover_bible_medium(story_folder: &str) -> String {
    let root = Path::new(story_folder);
    if !root.exists() {
        return String::new();
    }
    let structure = crate::folder_structure::current();
    let mut parts: Vec<String> = Vec::new();

    if let Some(dir) = crate::folder_structure::resolve_subdir(root, structure.characters()) {
        let content = read_md_folder(&dir);
        if !content.is_empty() {
            parts.push(format!("## Characters\n\n{content}"));
        }
    }
    if let Some(dir) = crate::folder_structure::resolve_subdir(root, structure.locations()) {
        let content = read_md_folder(&dir);
        if !content.is_empty() {
            parts.push(format!("## Locations\n\n{content}"));
        }
    }

    truncate_words_joined(&parts.join("\n\n---\n\n"), BIBLE_MEDIUM_WORD_LIMIT)
}

fn truncate_words_joined(text: &str, max_words: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= max_words {
        return text.to_string();
    }
    words[..max_words].join(" ") + "\n[Bible truncated]"
}

/// Auto-discover bible content from a story folder using Settings → Folder Structure:
///   1. Configured bible subfolder — all .md files concatenated
///   2. Configured characters subfolder — all .md files concatenated
///   3. bible.md or story-bible.md in the folder root
/// Returns the combined text, or empty string if nothing found.
pub fn discover_bible(story_folder: &str) -> String {
    let root = Path::new(story_folder);
    if !root.exists() { return String::new(); }

    let structure = crate::folder_structure::current();
    let mut parts: Vec<String> = Vec::new();

    if let Some(dir) = crate::folder_structure::resolve_subdir(root, structure.bible()) {
        let content = read_md_folder(&dir);
        if !content.is_empty() {
            parts.push(format!("## Story Bible\n\n{}", content));
        }
    }

    if let Some(dir) = crate::folder_structure::resolve_subdir(root, structure.characters()) {
        let content = read_md_folder(&dir);
        if !content.is_empty() {
            parts.push(format!("## Characters\n\n{}", content));
        }
    }

    if let Some(dir) = crate::folder_structure::resolve_subdir(root, structure.locations()) {
        let content = read_md_folder(&dir);
        if !content.is_empty() {
            parts.push(format!("## Locations\n\n{}", content));
        }
    }

    // Check for single bible file in root
    if parts.is_empty() {
        for name in &["bible.md", "story-bible.md", "Bible.md", "Story-Bible.md"] {
            let file = root.join(name);
            if file.is_file() {
                if let Ok(text) = std::fs::read_to_string(&file) {
                    parts.push(text);
                }
                break;
            }
        }
    }

    let combined = parts.join("\n\n---\n\n");

    // Truncate to full-tier limit
    truncate_words_joined(&combined, BIBLE_FULL_WORD_LIMIT)
}

/// Read all .md files in a folder, sorted by name, concatenated with separators.
fn read_md_folder(dir: &Path) -> String {
    let Ok(entries) = std::fs::read_dir(dir) else { return String::new() };

    let mut files: Vec<std::path::PathBuf> = entries
        .flatten()
        .filter(|e| {
            let p = e.path();
            p.is_file() && p.extension().map(|ext| ext == "md").unwrap_or(false)
        })
        .map(|e| e.path())
        .collect();

    files.sort();

    let mut parts = Vec::new();
    for file in files {
        if let Ok(text) = std::fs::read_to_string(&file) {
            if !text.trim().is_empty() {
                parts.push(text);
            }
        }
    }
    parts.join("\n\n")
}

/// Load bible for a story: try auto-discovery first, fall back to explicit path.
pub fn load_bible_for_story(story_folder: &str, explicit_bible_path: &str) -> String {
    // Try auto-discovery from folder structure
    let discovered = discover_bible(story_folder);
    if !discovered.is_empty() {
        return discovered;
    }

    // Fall back to explicit path
    load_bible(explicit_bible_path)
}

/// Load bible text from a single explicit file path. Returns empty string if not found.
pub fn load_bible(bible_path: &str) -> String {
    if bible_path.is_empty() { return String::new(); }
    let path = Path::new(bible_path);
    if !path.exists() { return String::new(); }
    match std::fs::read_to_string(path) {
        Ok(text) => truncate_words_joined(&text, BIBLE_FULL_WORD_LIMIT),
        Err(_) => String::new(),
    }
}

// ── Preprocessed text cache ───────────────────────────────────────────────────

/// Get or create preprocessed text for a chapter+report_type combo.
/// Returns cached version if the source file hasn't changed.
pub fn get_preprocessed(
    conn: &Connection,
    story_folder: &str,
    chapter_file: &str,
    report_type: &str,
    source_path: &Path,
) -> Option<String> {
    let file_mtime = get_file_mtime(source_path);

    // Check cache
    let cached: Option<(String, String)> = conn.query_row(
        "SELECT processed_text, source_modified_at FROM preprocessed_chapters
         WHERE story_folder = ?1 AND chapter_file = ?2 AND report_type = ?3",
        params![story_folder, chapter_file, report_type],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).ok();

    if let Some((text, cached_mtime)) = cached {
        if cached_mtime == file_mtime {
            return Some(text);
        }
    }
    None
}

/// Store preprocessed text in the cache.
pub fn store_preprocessed(
    conn: &Connection,
    story_folder: &str,
    chapter_file: &str,
    report_type: &str,
    processed_text: &str,
    source_path: &Path,
) {
    let file_mtime = get_file_mtime(source_path);
    let now = chrono::Utc::now().to_rfc3339();
    let _ = conn.execute(
        "INSERT INTO preprocessed_chapters (story_folder, chapter_file, report_type, processed_text, source_modified_at, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(story_folder, chapter_file, report_type)
         DO UPDATE SET processed_text = excluded.processed_text, source_modified_at = excluded.source_modified_at, created_at = excluded.created_at",
        params![story_folder, chapter_file, report_type, processed_text, file_mtime, now],
    );
}

fn get_file_mtime(path: &Path) -> String {
    path.metadata()
        .and_then(|m| m.modified())
        .map(|t| {
            let dur = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
            format!("{}", dur.as_secs())
        })
        .unwrap_or_default()
}

// ── Placeholder filling ───────────────────────────────────────────────────────

/// Fill placeholders in a template string. Placeholders are {key} format.
/// Any unfilled placeholder is replaced with empty string.
pub fn fill_template(template: &str, vars: &HashMap<&str, &str>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        result = result.replace(&format!("{{{}}}", key), value);
    }
    // Remove any unfilled placeholders
    let re_unfilled = regex::Regex::new(r"\{[a-z_]+\}").unwrap();
    re_unfilled.replace_all(&result, "").to_string()
}

// ── Execute a prompt ──────────────────────────────────────────────────────────

/// Full prompt execution: load template, fill variables, call LLM.
pub async fn execute_prompt(
    db: &Db,
    template_id: &str,
    provider: &str,
    api_key: &str,
    model: &str,
    vars: HashMap<&str, &str>,
) -> Result<String, String> {
    execute_prompt_for_story(db, template_id, provider, api_key, model, vars, None).await
}

/// Like `execute_prompt` but enables prompt-cache keying per story folder.
pub async fn execute_prompt_for_story(
    db: &Db,
    template_id: &str,
    provider: &str,
    api_key: &str,
    model: &str,
    vars: HashMap<&str, &str>,
    story_folder: Option<&str>,
) -> Result<String, String> {
    let template = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        load_template(&conn, template_id)?
    };

    let system_prompt = fill_template(&template.system_prompt, &vars);
    let user_content = fill_template(&template.user_template, &vars);
    let max_tokens = template.max_tokens;
    let cache_key = story_folder.map(|f| crate::llm::story_cache_key(template_id, f));
    let opts = crate::llm::LlmCallOpts {
        cache_key: cache_key.as_deref(),
        template_id: Some(template_id),
        db: Some(db),
        story_folder,
    };

    if template.json_mode {
        crate::llm::call_llm_json(provider, api_key, model, &system_prompt, &user_content, max_tokens, opts).await
    } else {
        crate::llm::call_llm(provider, api_key, model, &system_prompt, &user_content, max_tokens, opts).await
    }
}

// ── Chapter preprocessing functions ───────────────────────────────────────────

/// Preprocess chapter text for show-don't-tell checking.
pub fn preprocess_for_sdt(content: &str) -> String {
    truncate_words(content, crate::analysis::chapters::CRAFT_EXCERPT_WORD_LIMIT)
}

/// Preprocess chapter text for AI-isms checking.
pub fn preprocess_for_ai_isms(content: &str) -> String {
    truncate_words(content, crate::analysis::chapters::CRAFT_EXCERPT_WORD_LIMIT)
}

fn truncate_words(text: &str, max: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= max { return text.to_string(); }
    words[..max].join(" ")
}

// analysis/chapters.rs — Phase 1: per-chapter AI genre-signal summaries
//
// Change detection uses source_hash only. Summaries are prose stored in
// chapter_summaries.signals and fed to book-level genre analysis.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};

use super::chapter_stats::ChapterFingerprint;
use super::{emit, err, GenreResult, FolderRequest};
use crate::db;
use crate::prompts;

/// Max words of chapter text sent to the summary model (~full chapter for typical manuscripts).
pub const CHAPTER_SUMMARY_WORD_LIMIT: usize = 2000;

pub struct Phase1Config<'a> {
    pub provider:        &'a str,
    pub api_key:         &'a str,
    pub summaries_model: &'a str,
    pub default_model:   &'a str,
    pub force:           bool,
}

impl<'a> Phase1Config<'a> {
    pub fn resolve_summaries_model(&self) -> Result<String, String> {
        crate::ai::resolve_slot_model(self.summaries_model, self.default_model)
    }
}

pub fn phase1_config_from<'a>(
    provider: &'a str,
    api_key: &'a str,
    model: &'a str,
    summaries_model: &'a str,
    force: bool,
) -> Phase1Config<'a> {
    Phase1Config {
        provider,
        api_key,
        summaries_model,
        default_model: model,
        force,
    }
}

// ── Tauri command ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn generate_summaries(app: AppHandle, request: FolderRequest) -> GenreResult {
    let folder = PathBuf::from(&request.folder);
    if !folder.exists() {
        return err("Folder does not exist.");
    }

    crate::reset_cancel();
    let chapters = collect_chapters(&folder);
    if chapters.is_empty() {
        return err("No .md files found.");
    }

    let config = phase1_config_from(
        &request.provider,
        &request.api_key,
        &request.model,
        &request.summaries_model,
        false,
    );
    if let Err(msg) = validate_phase1_ai(&config) {
        return err(&msg);
    }

    emit(
        &app,
        &format!(
            "Found {} chapter file(s). Summarizing genre signals...",
            chapters.len()
        ),
    );

    let database = app.state::<db::Db>();
    let (done, skipped) =
        phase1_summaries(&app, &database, &chapters, &request.folder, &config).await;

    let run_ts = chrono::Utc::now().to_rfc3339();
    {
        let folder_path = PathBuf::from(&request.folder);
        let manuscript_fp = compute_manuscript_fingerprint(&folder_path);
        let conn = database.0.lock().unwrap();
        let _ = db::record_artifact_built(&conn, &request.folder, "summaries", &manuscript_fp);
        let _ = db::sync_manuscript_state(&conn, &request.folder, &manuscript_fp);
    }

    GenreResult {
        success: true,
        report: format!(
            "\u{2713} {} summarized, {} already up to date.",
            done, skipped
        ),
        error: String::new(),
        run_ts,
    }
}

fn validate_phase1_ai(config: &Phase1Config<'_>) -> Result<(), String> {
    let model = config.resolve_summaries_model()?;
    crate::ai::ai_ready(config.provider, config.api_key, &model)
}

// ── Phase 1 implementation ───────────────────────────────────────────────────

pub(crate) async fn phase1_summaries(
    app: &AppHandle,
    database: &db::Db,
    chapters: &[PathBuf],
    story_folder: &str,
    config: &Phase1Config<'_>,
) -> (usize, usize) {
    let mut done = 0usize;
    let mut skipped = 0usize;

    let summaries_model = match config.resolve_summaries_model() {
        Ok(m) => m,
        Err(e) => {
            emit(app, &format!("\u{26a0} {}", e));
            return (0, 0);
        }
    };
    if let Err(e) = crate::ai::ai_ready(config.provider, config.api_key, &summaries_model) {
        emit(app, &format!("\u{26a0} {}", e));
        return (0, 0);
    }

    for (i, chapter_path) in chapters.iter().enumerate() {
        let fname = chapter_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        emit(
            app,
            &format!("  [{}/{}] Summarizing: {}", i + 1, chapters.len(), fname),
        );
        emit_summary_progress(app, &fname, "started");

        let content = match fs::read_to_string(chapter_path) {
            Ok(c) if !c.trim().is_empty() => c,
            Ok(_) => {
                emit(app, "    \u{26a0} Empty \u{2014} skipping.");
                emit_summary_progress(app, &fname, "skipped");
                continue;
            }
            Err(e) => {
                emit(app, &format!("    \u{26a0} Read error: {}", e));
                emit_summary_progress(app, &fname, "skipped");
                continue;
            }
        };

        let cleaned_source = clean_for_ai(&content);
        if cleaned_source.is_empty() {
            emit(app, "    \u{26a0} Empty after cleanup \u{2014} skipping.");
            emit_summary_progress(app, &fname, "skipped");
            continue;
        }

        let source_hash = chapter_source_hash(&cleaned_source);
        if !config.force {
            let conn = database.0.lock().unwrap();
            if db::chapter_has_current_summary(&conn, story_folder, &fname, &source_hash) {
                emit(
                    app,
                    &format!("  [{}/{}] SKIP: {}", i + 1, chapters.len(), fname),
                );
                emit_summary_progress(app, &fname, "skipped");
                skipped += 1;
                continue;
            }
        }

        let title = extract_title(&content)
            .or_else(|| extract_title(&cleaned_source))
            .unwrap_or_else(|| fname.clone());
        let word_count = cleaned_source.split_whitespace().count();
        let chapter_text = truncate_words(&cleaned_source, CHAPTER_SUMMARY_WORD_LIMIT);

        emit(
            app,
            &format!(
                "    {} words (sending {} to {})...",
                word_count,
                chapter_text.split_whitespace().count(),
                summaries_model
            ),
        );

        match summarize_chapter(
            database,
            config.provider,
            config.api_key,
            &summaries_model,
            story_folder,
            &title,
            &chapter_text,
            word_count,
        )
        .await
        {
            Ok(signals) => {
                let conn = database.0.lock().unwrap();
                if let Err(e) = db::save_chapter_summary(
                    &conn,
                    story_folder,
                    &fname,
                    &title,
                    &signals,
                    &source_hash,
                    word_count as i64,
                ) {
                    emit(app, &format!("    \u{26a0} Save error: {}", e));
                } else {
                    emit(app, "    \u{2713} Summary saved");
                    emit_summary_progress(app, &fname, "done");
                    done += 1;
                }
            }
            Err(e) => emit(app, &format!("    \u{26a0} AI error: {}", e)),
        }

        if crate::is_cancelled() {
            emit(app, "\u{26a0} Cancelled.");
            break;
        }
    }

    emit(
        app,
        &format!(
            "Phase 1 complete \u{2014} {} summarized, {} skipped.",
            done, skipped
        ),
    );
    (done, skipped)
}

async fn summarize_chapter(
    database: &db::Db,
    provider: &str,
    api_key: &str,
    model: &str,
    story_folder: &str,
    title: &str,
    chapter_text: &str,
    word_count: usize,
) -> Result<String, String> {
    let bible = prompts::load_bible_for_story(story_folder, "");
    let computed = format!("Word count: {word_count}");
    let title_owned = title.to_string();
    let chapter_owned = chapter_text.to_string();
    let mut vars = HashMap::new();
    vars.insert("chapter_title", title_owned.as_str());
    vars.insert("computed_signals", computed.as_str());
    vars.insert("bible", bible.as_str());
    vars.insert("chapter_text", chapter_owned.as_str());
    let prose = prompts::execute_prompt(database, "chapter_summary", provider, api_key, model, vars).await?;
    let trimmed = prose.trim();
    if trimmed.is_empty() {
        return Err("Empty summary returned.".to_string());
    }
    Ok(trimmed.to_string())
}

fn emit_summary_progress(app: &AppHandle, filename: &str, status: &str) {
    let _ = app.emit(
        "summary:chapter-progress",
        serde_json::json!({ "filename": filename, "status": status }),
    );
}

pub(crate) fn clean_for_ai(text: &str) -> String {
    fn is_non_visible(c: char) -> bool {
        matches!(
            c,
            '\u{00AD}' | '\u{034F}' | '\u{061C}' | '\u{115F}' | '\u{1160}'
            | '\u{17B4}' | '\u{17B5}' | '\u{180E}' | '\u{200B}' | '\u{200C}'
            | '\u{200D}' | '\u{200E}' | '\u{200F}' | '\u{202A}' | '\u{202B}'
            | '\u{202C}' | '\u{202D}' | '\u{202E}' | '\u{2060}' | '\u{2066}'
            | '\u{2067}' | '\u{2068}' | '\u{2069}' | '\u{FEFF}'
        )
    }

    let mut cleaned = String::with_capacity(text.len());
    for c in text.chars() {
        if is_non_visible(c) {
            cleaned.push(' ');
            continue;
        }
        if c.is_control() && !matches!(c, '\n' | '\r' | '\t') {
            cleaned.push(' ');
            continue;
        }
        cleaned.push(c);
    }

    cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

/// Aggregate fingerprint over all manuscript chapters: sorted `file:hash` pairs, SHA-256.
pub(crate) fn compute_manuscript_fingerprint(folder: &Path) -> String {
    let chapters = collect_chapters(folder);
    let mut pairs: Vec<String> = Vec::new();
    for chapter_path in &chapters {
        let fname = chapter_path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        let source = fs::read_to_string(chapter_path).unwrap_or_default();
        let cleaned = clean_for_ai(&source);
        if cleaned.is_empty() {
            continue;
        }
        let hash = chapter_source_hash(&cleaned);
        pairs.push(format!("{fname}:{hash}"));
    }
    let mut hasher = Sha256::new();
    hasher.update(pairs.join("\n").as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for b in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

pub(crate) fn chapter_source_hash(cleaned_text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(cleaned_text.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for b in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

/// Collect chapter `.md` files from the configured manuscript folder only.
pub(crate) fn collect_chapters(folder: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let structure = crate::folder_structure::current();
    let Some(manuscript_dir) = crate::folder_structure::resolve_subdir(folder, structure.manuscript()) else {
        return files;
    };
    collect_md_recursive(&manuscript_dir, &mut files);
    files.sort_by(|a, b| {
        natural_sort_key(a.to_string_lossy().as_ref())
            .cmp(&natural_sort_key(b.to_string_lossy().as_ref()))
    });
    files
}

fn collect_md_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect_md_recursive(&path, out);
        } else if path.extension().map(|e| e == "md").unwrap_or(false) {
            out.push(path);
        }
    }
}

fn natural_sort_key(s: &str) -> Vec<u64> {
    let mut key = Vec::new();
    let mut cur = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            cur.push(c);
        } else {
            if !cur.is_empty() {
                key.push(cur.parse::<u64>().unwrap_or(0));
                cur.clear();
            }
            key.push(c as u64);
        }
    }
    if !cur.is_empty() {
        key.push(cur.parse::<u64>().unwrap_or(0));
    }
    key
}

pub(crate) fn extract_title(content: &str) -> Option<String> {
    content
        .lines()
        .take(10)
        .find(|l| l.trim().starts_with("# "))
        .map(|l| l.trim().trim_start_matches("# ").trim().to_string())
}

pub(crate) fn truncate_words(text: &str, max: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= max {
        return text.to_string();
    }
    words[..max].join(" ") + "\n\n[Truncated]"
}

/// True when `signals` holds AI prose, not a legacy fingerprint JSON blob.
pub fn is_prose_summary(signals: &str) -> bool {
    let s = signals.trim();
    if s.is_empty() {
        return false;
    }
    if ChapterFingerprint::from_storage(s).is_some() {
        return false;
    }
    true
}

/// Book-level dossier from per-chapter AI genre-signal summaries.
pub(crate) fn build_combined_context(summaries: &[db::ChapterSummaryRow]) -> String {
    let mut out = String::from(
        "Chapter genre-signal summaries for the full manuscript.\n\
         Use these to infer genre niche, subgenre, tone, faith vs secular content, heat level, and category fit.\n\n",
    );

    for (i, s) in summaries.iter().enumerate() {
        let body = s.signals.trim();
        if body.is_empty() {
            continue;
        }
        out.push_str(&format!(
            "--- Chapter {} — {} (~{} words) ---\n{}\n\n",
            i + 1,
            s.title,
            s.word_count,
            body
        ));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_for_ai_strips_non_visible_chars() {
        let raw = "Hello\u{200B}world\u{FEFF}";
        let cleaned = clean_for_ai(raw);
        assert_eq!(cleaned, "Hello world");
    }

    #[test]
    fn hash_ignores_non_visible_char_differences() {
        let a = clean_for_ai("The night\u{200B}was cold.");
        let b = clean_for_ai("The night was cold.");
        assert_eq!(chapter_source_hash(&a), chapter_source_hash(&b));
    }

    #[test]
    fn build_combined_context_uses_prose_summaries() {
        let row = db::ChapterSummaryRow {
            file: "01.md".into(),
            title: "Ch1".into(),
            signals: "Contemporary romantic suspense. No Christian or faith themes.".into(),
            word_count: 1500,
        };
        let combined = build_combined_context(&[row]);
        assert!(combined.contains("romantic suspense"));
        assert!(combined.contains("No Christian"));
    }

    #[test]
    fn is_prose_summary_rejects_fingerprint_json() {
        let fp = super::super::chapter_stats::compute_chapter_fingerprint("T", "text");
        assert!(!is_prose_summary(&fp.to_storage_json()));
        assert!(is_prose_summary("Romance with suspense elements."));
    }
}

// analysis/chapters.rs — Phase 1: chapter-by-chapter summarization
//
// Collects .md files from a manuscript folder, sends each to an LLM for
// genre-signal extraction, and persists the results to SQLite.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager};

use super::{emit, err, GenreResult, FolderRequest};
use crate::db;
use crate::prompts;

// ── Tauri command ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn generate_summaries(app: AppHandle, request: FolderRequest) -> GenreResult {
    let folder = PathBuf::from(&request.folder);
    if !folder.exists() { return err("Folder does not exist."); }

    crate::reset_cancel();
    let chapters = collect_chapters(&folder);
    if chapters.is_empty() { return err("No .md files found."); }

    emit(&app, &format!("Found {} chapter file(s). Starting summaries...", chapters.len()));

    let database = app.state::<db::Db>();
    let (done, skipped) = phase1_summaries(&app, &database, &chapters, &request.folder, &request.provider, &request.api_key, &request.model).await;

    GenreResult {
        success: true,
        report:  format!("\u{2713} {} summarized, {} already done.", done, skipped),
        error:   String::new(),
        run_ts:  String::new(),
    }
}

// ── Phase 1 implementation ───────────────────────────────────────────────────

pub(crate) async fn phase1_summaries(
    app: &AppHandle,
    database: &db::Db,
    chapters: &[PathBuf],
    story_folder: &str,
    provider: &str,
    api_key: &str,
    model: &str,
) -> (usize, usize) {
    let mut done = 0usize;
    let mut skipped = 0usize;
    let summary_hashes = {
        let conn = database.0.lock().unwrap();
        db::load_chapter_summary_hashes(&conn, story_folder)
    };

    for (i, chapter_path) in chapters.iter().enumerate() {
        let fname = chapter_path.file_name().unwrap_or_default().to_string_lossy().to_string();

        emit(app, &format!("  [{}/{}] Summarizing: {}", i + 1, chapters.len(), fname));

        let content = match fs::read_to_string(chapter_path) {
            Ok(c) if !c.trim().is_empty() => c,
            Ok(_)  => { emit(app, "    \u{26a0} Empty \u{2014} skipping."); continue; }
            Err(e) => { emit(app, &format!("    \u{26a0} Read error: {}", e)); continue; }
        };

        let cleaned_source = clean_for_ai(&content);
        if cleaned_source.is_empty() {
            emit(app, "    \u{26a0} Empty after cleanup \u{2014} skipping.");
            continue;
        }

        let source_hash = chapter_source_hash(&cleaned_source);
        if summary_hashes.get(&fname).map(|h| h == &source_hash).unwrap_or(false) {
            emit(app, &format!("  [{}/{}] SKIP: {}", i + 1, chapters.len(), fname));
            skipped += 1;
            continue;
        }

        let word_count = cleaned_source.split_whitespace().count();
        emit(app, &format!("    {} words", word_count));

        match summarize_chapter(database, provider, api_key, model, story_folder, &fname, &truncate_words(&cleaned_source, 8000)).await {
            Ok(signals) => {
                let title = extract_title(&content).unwrap_or_else(|| fname.clone());
                let compact_signals = clean_for_ai(&signals);
                let conn = database.0.lock().unwrap();
                let _ = db::save_chapter_summary(&conn, story_folder, &fname, &title, &compact_signals, &source_hash, word_count as i64);
                emit(app, &format!("    \u{2713} Done ({} signal chars)", compact_signals.len()));
                done += 1;
            }
            Err(e) => emit(app, &format!("    \u{26a0} AI error: {}", e)),
        }

        if crate::is_cancelled() { emit(app, "\u{26a0} Cancelled."); break; }
    }

    emit(app, &format!("Phase 1 complete \u{2014} {} new, {} skipped.", done, skipped));
    (done, skipped)
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
            continue;
        }
        if c.is_control() && !matches!(c, '\n' | '\r' | '\t') {
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

// ── AI call ──────────────────────────────────────────────────────────────────

pub(crate) async fn summarize_chapter(
    db: &db::Db,
    provider: &str,
    api_key: &str,
    model: &str,
    story_folder: &str,
    filename: &str,
    content: &str,
) -> Result<String, String> {
    let bible = prompts::discover_bible(story_folder);
    let title = extract_title(content).unwrap_or_else(|| filename.to_string());

    let mut vars = HashMap::new();
    vars.insert("chapter_title", title.as_str());
    vars.insert("chapter_text", content);
    vars.insert("bible", bible.as_str());

    prompts::execute_prompt(db, "chapter_summary", provider, api_key, model, vars).await
}

// ── File helpers (manuscript source files only — these stay on disk) ──────────

/// Collect chapter `.md` files from the configured manuscript folder only.
/// Other story folders (bible, characters, publishing, etc.) are ignored.
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
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if name.starts_with('.') { continue; }
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
    content.lines().take(10)
        .find(|l| l.trim().starts_with("# "))
        .map(|l| l.trim().trim_start_matches("# ").trim().to_string())
}

pub(crate) fn truncate_words(text: &str, max: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= max { return text.to_string(); }
    words[..max].join(" ") + "\n\n[Truncated]"
}

pub(crate) fn build_combined_context(summaries: &[db::ChapterSummaryRow]) -> String {
    summaries.iter().enumerate().map(|(i, s)| {
        format!("--- Chapter {} ({}, ~{} words) ---\n{}\n\n", i + 1, s.title, s.word_count, s.signals)
    }).collect()
}

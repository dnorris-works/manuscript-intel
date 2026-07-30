// analysis/chapters.rs — Phase 1: chapter-by-chapter manuscript fingerprinting
//
// Scans every chapter with deterministic Rust heuristics (no per-chapter AI).
// Fingerprints are stored as JSON and aggregated for book-level genre analysis.

use std::path::{Path, PathBuf};
use std::fs;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};

use super::{emit, err, GenreResult, FolderRequest};
use super::chapter_stats::{aggregate_lexicon, ChapterFingerprint};
use crate::db;

// ── Tauri command ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn generate_summaries(app: AppHandle, request: FolderRequest) -> GenreResult {
    let folder = PathBuf::from(&request.folder);
    if !folder.exists() { return err("Folder does not exist."); }

    crate::reset_cancel();
    let chapters = collect_chapters(&folder);
    if chapters.is_empty() { return err("No .md files found."); }

    emit(&app, &format!(
        "Found {} chapter file(s). Scanning fingerprints (no AI)...",
        chapters.len()
    ));

    let database = app.state::<db::Db>();
    let (done, skipped) = phase1_summaries(&app, &database, &chapters, &request.folder).await;

    let run_ts = chrono::Utc::now().to_rfc3339();
    {
        let folder_path = PathBuf::from(&request.folder);
        let manuscript_fp = compute_manuscript_fingerprint(&folder_path);
        let conn = database.0.lock().unwrap();
        let _ = db::record_artifact_built(&conn, &request.folder, "fingerprints", &manuscript_fp);
        let _ = db::sync_manuscript_state(&conn, &request.folder, &manuscript_fp);
    }

    GenreResult {
        success: true,
        report:  format!("\u{2713} {} scanned, {} already up to date.", done, skipped),
        error:   String::new(),
        run_ts,
    }
}

// ── Phase 1 implementation ───────────────────────────────────────────────────

pub(crate) async fn phase1_summaries(
    app: &AppHandle,
    database: &db::Db,
    chapters: &[PathBuf],
    story_folder: &str,
) -> (usize, usize) {
    let mut done = 0usize;
    let mut skipped = 0usize;
    let summary_hashes = {
        let conn = database.0.lock().unwrap();
        db::load_chapter_summary_hashes(&conn, story_folder)
    };

    for (i, chapter_path) in chapters.iter().enumerate() {
        let fname = chapter_path.file_name().unwrap_or_default().to_string_lossy().to_string();

        emit(app, &format!("  [{}/{}] Scanning: {}", i + 1, chapters.len(), fname));
        emit_summary_progress(app, &fname, "started");

        let content = match fs::read_to_string(chapter_path) {
            Ok(c) if !c.trim().is_empty() => c,
            Ok(_)  => {
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
        let skip = match summary_hashes.get(&fname) {
            Some(h) if !h.is_empty() && h == &source_hash => {
                let conn = database.0.lock().unwrap();
                db::chapter_fingerprint_complete(&conn, story_folder, &fname)
            }
            _ => false,
        };
        if skip {
            emit(app, &format!("  [{}/{}] SKIP: {}", i + 1, chapters.len(), fname));
            emit_summary_progress(app, &fname, "skipped");
            skipped += 1;
            continue;
        }

        if summary_hashes.get(&fname).is_some() {
            emit(app, &format!("  [{}/{}] Re-scanning: {}", i + 1, chapters.len(), fname));
        }

        let title = extract_title(&content)
            .or_else(|| extract_title(&cleaned_source))
            .unwrap_or_else(|| fname.clone());
        let fingerprint = super::chapter_stats::compute_chapter_fingerprint(&title, &cleaned_source);
        let word_count = fingerprint.word_count;

        emit(app, &format!("    {} words — {}", word_count, fingerprint.to_display_summary()));

        let conn = database.0.lock().unwrap();
        let _ = db::save_chapter_fingerprint(
            &conn,
            story_folder,
            &fname,
            &fingerprint,
            &source_hash,
        );
        emit(app, "    \u{2713} Fingerprint saved");
        emit_summary_progress(app, &fname, "done");
        done += 1;

        if crate::is_cancelled() { emit(app, "\u{26a0} Cancelled."); break; }
    }

    emit(app, &format!("Phase 1 complete \u{2014} {} scanned, {} skipped.", done, skipped));
    (done, skipped)
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

// ── File helpers (manuscript source files only — these stay on disk) ──────────

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

/// Format stored signals (JSON fingerprint or legacy prose) for one dossier entry.
pub fn format_chapter_for_dossier(signals: &str) -> String {
    if let Some(fp) = ChapterFingerprint::from_storage(signals) {
        return fp.to_dossier_line();
    }
    signals.trim().to_string()
}

/// Book-level dossier from all chapter fingerprints for genre analysis (TokenMix).
pub(crate) fn build_combined_context(summaries: &[db::ChapterSummaryRow]) -> String {
    let mut fingerprints: Vec<ChapterFingerprint> = Vec::new();
    let mut out = String::from(
        "Structured manuscript fingerprint (deterministic scan of every chapter).\n\
         Infer the true genre niche, subgenre, tone, and category fit from these signals.\n\n",
    );

    for (i, s) in summaries.iter().enumerate() {
        let body = format_chapter_for_dossier(&s.signals);
        out.push_str(&format!(
            "--- Chapter {} — {} (~{} words) ---\n{}\n\n",
            i + 1,
            s.title,
            s.word_count,
            body
        ));
        if let Some(fp) = ChapterFingerprint::from_storage(&s.signals) {
            fingerprints.push(fp);
        }
    }

    if !fingerprints.is_empty() {
        let totals = aggregate_lexicon(&fingerprints);
        if !totals.is_empty() {
            let mut pairs: Vec<(String, u32)> = totals.into_iter().collect();
            pairs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            let lex_line: String = pairs
                .iter()
                .map(|(k, v)| format!("{k} ({v})"))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str("--- Book-level lexicon totals (all chapters) ---\n");
            out.push_str(&lex_line);
            out.push('\n');
        }
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
    fn build_combined_context_includes_lexicon_totals() {
        let fp = super::super::chapter_stats::compute_chapter_fingerprint(
            "Ch1",
            "She prayed in church and fell in love.",
        );
        let row = db::ChapterSummaryRow {
            file: "01.md".into(),
            title: "Ch1".into(),
            signals: fp.to_storage_json(),
            word_count: 8,
        };
        let combined = build_combined_context(&[row]);
        assert!(combined.contains("faith"));
        assert!(combined.contains("Book-level lexicon totals"));
    }
}

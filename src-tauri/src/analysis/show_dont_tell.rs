// analysis/show_dont_tell.rs — AI-assisted "Show Don't Tell" checker.
//
// Sends each chapter to the LLM asking it to identify passages where the
// author *tells* the reader something (emotions, reactions, judgments) rather
// than *showing* through action, dialogue, or sensory detail.
//
// The report includes the offending text plus surrounding context so the
// author can see exactly where the problem is.

use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use super::{emit, err, GenreResult};
use super::chapters::{collect_chapters, extract_title};
use crate::batch_prompt::{self, BatchChapterItem, CRAFT_BATCH_WORD_BUDGET};
use crate::db;

// ── Request ──────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct ShowDontTellRequest {
    pub folder:   String,
    pub provider: String,
    pub api_key:  String,
    pub model:    String,
    #[serde(default)]
    pub bible_path: String,
}

// ── AI response shape ────────────────────────────────────────────────────────

#[derive(serde::Deserialize, Clone, Debug)]
struct AiViolation {
    #[serde(default)]
    telling_text: String,
    #[serde(default)]
    context:      String,
    #[serde(default)]
    why:          String,
    #[serde(default)]
    severity:     String,  // "minor" | "moderate" | "major"
}

// ── Command ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn check_show_dont_tell(app: AppHandle, request: ShowDontTellRequest) -> GenreResult {
    let cancel = crate::cancel_notify();
    tokio::select! {
        result = check_inner(app, request) => result,
        _ = cancel.notified() => err("Cancelled."),
    }
}

async fn check_inner(app: AppHandle, request: ShowDontTellRequest) -> GenreResult {
    let folder = PathBuf::from(&request.folder);
    if !folder.exists() { return err("Folder does not exist."); }
    if let Err(msg) = crate::ai::ai_ready(&request.provider, &request.api_key, &request.model) {
        return err(&msg);
    }

    crate::reset_cancel();
    let database = app.state::<db::Db>();
    let run_ts = chrono::Utc::now().to_rfc3339();

    let chapters = collect_chapters(&folder);
    if chapters.is_empty() { return err("No .md chapter files found."); }

    let bible = crate::prompts::load_bible_for_story(&request.folder, &request.bible_path);

    emit(&app, &format!("Checking {} chapter(s) for show-don't-tell violations...", chapters.len()));

    let mut chapter_meta: Vec<(usize, String, String)> = Vec::new();
    let mut batch_items: Vec<BatchChapterItem> = Vec::new();

    for (i, path) in chapters.iter().enumerate() {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let filename = path
            .strip_prefix(&folder)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        let title = extract_title(&content).unwrap_or_else(|| filename.clone());

        let processed = {
            let conn = database.0.lock().unwrap();
            crate::prompts::get_preprocessed(&conn, &request.folder, &filename, "sdt_check", path)
                .unwrap_or_else(|| {
                    let p = crate::prompts::preprocess_for_sdt(&content);
                    crate::prompts::store_preprocessed(&conn, &request.folder, &filename, "sdt_check", &p, path);
                    p
                })
        };

        chapter_meta.push((i, filename.clone(), title.clone()));
        batch_items.push(BatchChapterItem {
            file: filename,
            title,
            text: processed,
        });
    }

    let results = batch_prompt::process_chapters_batched(
        &database,
        &request.provider,
        &request.api_key,
        &request.model,
        "sdt_check_batch",
        "sdt_check",
        &bible,
        batch_items,
        CRAFT_BATCH_WORD_BUDGET,
        &[],
    )
    .await;

    let mut all_findings: Vec<serde_json::Value> = Vec::new();
    let mut total_violations = 0usize;

    for (i, filename, title) in chapter_meta {
        if crate::is_cancelled() {
            return err("Cancelled.");
        }

        let violations = match results.get(&filename) {
            Some(value) => parse_violations(value),
            None => {
                emit(&app, &format!("  ⚠ {}: no response", filename));
                Vec::new()
            }
        };

        if violations.is_empty() {
            emit(&app, &format!("  ✓ {} — clean", filename));
        } else {
            emit(&app, &format!("  → {} — {} violation(s)", filename, violations.len()));
            total_violations += violations.len();

            all_findings.push(serde_json::json!({
                "file": filename,
                "title": title,
                "chapter_index": i,
                "violations": violations.iter().map(|v| serde_json::json!({
                    "telling_text": v.telling_text,
                    "context": v.context,
                    "why": v.why,
                    "severity": v.severity,
                })).collect::<Vec<_>>(),
            }));
        }
    }

    emit(&app, &format!("✓ Show Don't Tell complete — {} violation(s) across {} chapter(s).",
        total_violations, all_findings.len()));

    // Build and save report
    let report = serde_json::json!({
        "schema": "show_dont_tell_v1",
        "note": "AI-assisted: the model identifies passages that tell instead of show. Severity is subjective — use as a prompt to revisit, not a verdict.",
        "summary": {
            "chapters_checked": chapters.len(),
            "chapters_with_violations": all_findings.len(),
            "total_violations": total_violations,
        },
        "chapters": all_findings,
    }).to_string();

    {
        let conn = database.0.lock().unwrap();
        let _ = db::save_document_at(&conn, &request.folder, "show_dont_tell", &report, &run_ts);
    }

    GenreResult { success: true, report: String::new(), error: String::new(), run_ts }
}

// ── AI extraction ────────────────────────────────────────────────────────────

fn parse_violations(value: &serde_json::Value) -> Vec<AiViolation> {
    batch_prompt::chapter_array_field(value, "findings")
        .into_iter()
        .filter_map(|item| serde_json::from_value::<AiViolation>(item).ok())
        .filter(|v| !v.telling_text.is_empty())
        .collect()
}

// ── Suggest fix for a show-don't-tell violation ──────────────────────────────

#[derive(serde::Deserialize)]
pub struct SuggestSdtFixRequest {
    pub provider:      String,
    pub api_key:       String,
    pub model:         String,
    pub telling_text:  String,
    pub context:       String,
    pub why:           String,
    pub chapter_title: String,
    #[serde(default)]
    pub folder:        String,
    #[serde(default)]
    pub bible_path:    String,
}

#[derive(serde::Serialize)]
pub struct SuggestSdtFixResult {
    pub success:     bool,
    pub suggestions: String,
    pub error:       String,
}

#[tauri::command]
pub async fn suggest_sdt_fix(app: AppHandle, request: SuggestSdtFixRequest) -> SuggestSdtFixResult {
    use std::collections::HashMap;

    let database = app.state::<db::Db>();
    let bible = crate::prompts::load_bible_for_story(&request.folder, &request.bible_path);

    let mut vars = HashMap::new();
    vars.insert("chapter_title", request.chapter_title.as_str());
    vars.insert("telling_text", request.telling_text.as_str());
    vars.insert("context", request.context.as_str());
    vars.insert("why", request.why.as_str());
    vars.insert("bible", bible.as_str());

    match crate::prompts::execute_prompt(
        &database, "sdt_suggest", &request.provider, &request.api_key, &request.model, vars,
    ).await {
        Ok(suggestions) => SuggestSdtFixResult { success: true, suggestions, error: String::new() },
        Err(e) => SuggestSdtFixResult { success: false, suggestions: String::new(), error: e },
    }
}

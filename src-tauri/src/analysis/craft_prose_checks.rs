// analysis/craft_prose_checks.rs — Combined Show Don't Tell + AI-isms in one batched pass.

use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use super::chapters::{chapter_source_hash, collect_chapters, extract_title};
use super::{emit, err, GenreResult};
use crate::batch_prompt::{self, BatchChapterItem, CachedBatchItem, CRAFT_BATCH_WORD_BUDGET};
use crate::db;
use crate::prompts::{self, BibleTier};

#[derive(serde::Deserialize)]
pub struct CraftProseChecksRequest {
    pub folder:   String,
    pub provider: String,
    pub api_key:  String,
    pub model:    String,
    #[serde(default)]
    pub bible_path: String,
}

#[derive(serde::Deserialize, Clone, Debug)]
struct AiViolation {
    #[serde(default)]
    telling_text: String,
    #[serde(default)]
    context:      String,
    #[serde(default)]
    why:          String,
    #[serde(default)]
    severity:     String,
}

#[tauri::command]
pub async fn check_craft_prose_combined(app: AppHandle, request: CraftProseChecksRequest) -> GenreResult {
    let cancel = crate::cancel_notify();
    tokio::select! {
        result = check_inner(app, request) => result,
        _ = cancel.notified() => err("Cancelled."),
    }
}

async fn check_inner(app: AppHandle, request: CraftProseChecksRequest) -> GenreResult {
    let folder = PathBuf::from(&request.folder);
    if !folder.exists() {
        return err("Folder does not exist.");
    }
    if let Err(msg) = crate::ai::ai_ready(&request.provider, &request.api_key, &request.model) {
        return err(&msg);
    }

    crate::reset_cancel();
    let database = app.state::<db::Db>();
    let run_ts = chrono::Utc::now().to_rfc3339();

    let chapters = collect_chapters(&folder);
    if chapters.is_empty() {
        return err("No .md chapter files found.");
    }

    let bible = prompts::load_bible_tiered(&request.folder, &request.bible_path, BibleTier::Minimal);

    emit(
        &app,
        &format!(
            "Checking {} chapter(s) for show-don't-tell and AI-isms (combined pass)...",
            chapters.len()
        ),
    );

    let mut chapter_meta: Vec<(usize, String, String)> = Vec::new();
    let mut batch_items: Vec<CachedBatchItem> = Vec::new();

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
            crate::prompts::get_preprocessed(&conn, &request.folder, &filename, "craft_prose_checks", path)
                .unwrap_or_else(|| {
                    let p = prompts::preprocess_for_sdt(&content);
                    crate::prompts::store_preprocessed(
                        &conn,
                        &request.folder,
                        &filename,
                        "craft_prose_checks",
                        &p,
                        path,
                    );
                    p
                })
        };

        chapter_meta.push((i, filename.clone(), title.clone()));
        batch_items.push(CachedBatchItem {
            item: BatchChapterItem {
                file: filename,
                title,
                text: processed,
            },
            source_hash: chapter_source_hash(&content),
        });
    }

    let results = batch_prompt::process_chapters_batched(
        &database,
        &request.provider,
        &request.api_key,
        &request.model,
        "craft_prose_checks_batch",
        "craft_prose_checks_single",
        &bible,
        &request.folder,
        Some("craft_prose_checks"),
        batch_items,
        CRAFT_BATCH_WORD_BUDGET,
        &[],
    )
    .await;

    let mut sdt_findings: Vec<serde_json::Value> = Vec::new();
    let mut ai_findings: Vec<serde_json::Value> = Vec::new();
    let mut total_sdt = 0usize;
    let mut total_ai = 0usize;

    for (i, filename, title) in chapter_meta {
        if crate::is_cancelled() {
            return err("Cancelled.");
        }

        let value = match results.get(&filename) {
            Some(v) => v,
            None => {
                emit(&app, &format!("  ⚠ {}: no response", filename));
                continue;
            }
        };

        let sdt_violations = parse_violations(value, "sdt_findings");
        let ai_violations = parse_violations(value, "ai_isms_findings");

        if sdt_violations.is_empty() && ai_violations.is_empty() {
            emit(&app, &format!("  ✓ {} — clean", filename));
        } else {
            emit(
                &app,
                &format!(
                    "  → {} — {} SDT, {} AI-ism(s)",
                    filename,
                    sdt_violations.len(),
                    ai_violations.len()
                ),
            );
        }

        total_sdt += sdt_violations.len();
        total_ai += ai_violations.len();

        if !sdt_violations.is_empty() {
            sdt_findings.push(serde_json::json!({
                "file": filename,
                "title": title,
                "chapter_index": i,
                "violations": violations_json(&sdt_violations),
            }));
        }
        if !ai_violations.is_empty() {
            ai_findings.push(serde_json::json!({
                "file": filename,
                "title": title,
                "chapter_index": i,
                "violations": violations_json(&ai_violations),
            }));
        }
    }

    emit(
        &app,
        &format!(
            "✓ Combined craft check — {} SDT + {} AI-ism flag(s).",
            total_sdt, total_ai
        ),
    );

    let sdt_report = serde_json::json!({
        "schema": "show_dont_tell_v1",
        "note": "AI-assisted: the model identifies passages that tell instead of show. Severity is subjective — use as a prompt to revisit, not a verdict.",
        "summary": {
            "chapters_checked": chapters.len(),
            "chapters_with_violations": sdt_findings.len(),
            "total_violations": total_sdt,
        },
        "chapters": sdt_findings,
    })
    .to_string();

    let ai_report = serde_json::json!({
        "schema": "ai_isms_v1",
        "note": "AI-assisted: the model flags prose habits that often read as machine-generated. Severity is subjective — use as a prompt to revise, not a verdict.",
        "summary": {
            "chapters_checked": chapters.len(),
            "chapters_with_violations": ai_findings.len(),
            "total_violations": total_ai,
        },
        "chapters": ai_findings,
    })
    .to_string();

    {
        let conn = database.0.lock().unwrap();
        let _ = db::save_document_at(&conn, &request.folder, "show_dont_tell", &sdt_report, &run_ts);
        let _ = db::save_document_at(&conn, &request.folder, "ai_isms", &ai_report, &run_ts);
    }

    GenreResult {
        success: true,
        report: String::new(),
        error: String::new(),
        run_ts,
    }
}

fn parse_violations(value: &serde_json::Value, field: &str) -> Vec<AiViolation> {
    batch_prompt::chapter_array_field(value, field)
        .into_iter()
        .filter_map(|item| serde_json::from_value::<AiViolation>(item).ok())
        .filter(|v| !v.telling_text.is_empty())
        .collect()
}

fn violations_json(violations: &[AiViolation]) -> Vec<serde_json::Value> {
    violations
        .iter()
        .map(|v| {
            serde_json::json!({
                "telling_text": v.telling_text,
                "context": v.context,
                "why": v.why,
                "severity": v.severity,
            })
        })
        .collect()
}

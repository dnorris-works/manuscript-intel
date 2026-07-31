// analysis/bisac.rs — BISAC subject-code classification
//
// BISAC is the industry-standard subject code system (maintained by BISG)
// submitted as metadata for KDP Print and any wide/Ingram distribution.
// Convention is max 3 codes per book, primary first.

use std::collections::HashMap;
use serde::Deserialize;
use tauri::{AppHandle, Manager};

use super::{emit, err, GenreResult, FolderRequest};
use crate::db;
use crate::prompts;

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct AiBisacPick { code: String, confidence: u8, reason: String }

// ── Tauri command ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn classify_bisac_for_story(app: AppHandle, request: FolderRequest) -> GenreResult {
    let database = app.state::<db::Db>();

    let genre_data = { let conn = database.0.lock().unwrap(); db::load_genre_data(&conn, &request.folder) };
    let genre_data = match genre_data {
        Some(d) => d,
        None    => return err("No genre data found. Run Analyze first."),
    };

    let master_list = { let conn = database.0.lock().unwrap(); db::master_bisac_list(&conn) };
    if master_list.is_empty() { return err("No BISAC codes loaded in the database."); }

    emit(&app, "Classifying against BISAC subject headings...");
    emit(&app, &format!("  Scoring against {} known codes.", master_list.len()));

    let description = format!("{}\n\n{}", genre_data.industry_ebook, genre_data.genre_signals);

    match ai_pick_bisac(&database, &request.provider, &request.api_key, &request.model, &description, &master_list).await {
        Err(e) => err(&e),
        Ok(picks) => {
            if picks.is_empty() { return err("AI did not select any BISAC codes."); }

            for p in &picks { emit(&app, &format!("  {}% — {} {}", p.2, p.0, p.1)); }

            let now_disp = chrono::Utc::now().format("%B %-d, %Y %H:%M UTC").to_string();
            let mut lines = vec![
                "# BISAC Classification".to_string(),
                format!("Generated: {}", now_disp),
                String::new(),
                "> **Verify before use.** These codes are AI-selected from a hand-seeded reference list, not a live BISG feed. Spot-check every code against BISG's free lookup (bisg.org/complete-bisac-subject-headings-list) before submitting to KDP Print or IngramSpark.".to_string(),
                String::new(),
                "BISAC convention: use up to 3 codes, primary listed first. This is separate from your Amazon KDP browse categories — Kindle eBook no longer takes BISAC directly (Amazon derives it from your browse category), but KDP Print and Ingram still require it explicitly.".to_string(),
                String::new(),
                "**On discoverability:** unlike KDP categories, there is no live data source for BISAC — no tool covers it, and Amazon has no browse mechanism for it. When two codes are close in fit, a more specific heading is preferred over a generic \"/ General\" one as a structural best-practice, not measured data.".to_string(),
                String::new(),
                "---".to_string(),
                String::new(),
            ];
            for (i, (code, heading, confidence, reason)) in picks.iter().enumerate() {
                lines.push(format!("## {}. `{}` — {}", i + 1, code, heading));
                lines.push(String::new());
                lines.push(format!("**{}% confidence** — {}", confidence, reason));
                lines.push(String::new());
            }

            let report = lines.join("\n");

            let conn = database.0.lock().unwrap();
            let rows: Vec<(String, String, u8, String)> = picks.iter()
                .map(|(code, heading, conf, reason)| (code.clone(), heading.clone(), *conf, reason.clone()))
                .collect();
            if let Err(e) = db::replace_bisac_classifications(&conn, &request.folder, "ebook", &rows) {
                emit(&app, &format!("  ⚠ Could not save BISAC classification to database: {}", e));
            }
            let _ = db::save_document(&conn, &request.folder, "bisac_classification", &report);
            emit(&app, &format!("✓ BISAC classification saved to database — {} code(s).", picks.len()));

            GenreResult { success: true, report, error: String::new(), run_ts: String::new() }
        }
    }
}

// ── Core logic ───────────────────────────────────────────────────────────────

pub(crate) async fn ai_pick_bisac(
    database: &db::Db,
    provider: &str,
    api_key: &str,
    model: &str,
    description: &str,
    master_list: &[db::BisacCodeRow],
) -> Result<Vec<(String, String, u8, String)>, String>
{
    let bisac_list = master_list.iter()
        .map(|c| format!("{} — {}", c.code, c.heading))
        .collect::<Vec<_>>()
        .join("\n");

    let mut vars = HashMap::new();
    vars.insert("bisac_list", bisac_list.as_str());
    vars.insert("description", description);

    let raw = prompts::execute_prompt(database, "bisac_pick", provider, api_key, model, vars).await?;
    let clean = raw.trim()
        .trim_start_matches("```json").trim_start_matches("```")
        .trim_end_matches("```").trim();

    let picks: Vec<AiBisacPick> = serde_json::from_str(clean)
        .map_err(|e| format!("Parse error (BISAC): {} | got: {}", e, &clean[..clean.len().min(300)]))?;

    let resolved: Vec<(String, String, u8, String)> = picks.into_iter().filter_map(|p| {
        master_list.iter().find(|c| c.code == p.code)
            .map(|c| (c.code.clone(), c.heading.clone(), p.confidence, p.reason))
    }).collect();

    Ok(resort_bisac_for_specificity(resolved))
}

/// Classify BISAC for ebook and/or print, persist rows, return JSON section text.
pub(crate) async fn run_bisac_classification(
    database: &db::Db,
    story_folder: &str,
    genre_data: &db::GenreDataRow,
    provider: &str,
    api_key: &str,
    model: &str,
    include_ebook: bool,
    include_print: bool,
) -> String {
    let bisac_master = { let conn = database.0.lock().unwrap(); db::master_bisac_list(&conn) };
    if bisac_master.is_empty() {
        return String::new();
    }

    let same_as_ebook = genre_data.industry_print.trim().eq_ignore_ascii_case(genre_data.industry_ebook.trim());
    let mut ebook_picks: Vec<(String, String, u8, String)> = Vec::new();

    if include_ebook {
        let ebook_desc = format!("{}\n\n{}", genre_data.industry_ebook, genre_data.genre_signals);
        ebook_picks = ai_pick_bisac(database, provider, api_key, model, &ebook_desc, &bisac_master)
            .await
            .unwrap_or_default();
        let conn = database.0.lock().unwrap();
        let rows: Vec<(String, String, u8, String)> = ebook_picks
            .iter()
            .map(|(c, h, cf, r)| (c.clone(), h.clone(), *cf, r.clone()))
            .collect();
        let _ = db::replace_bisac_classifications(&conn, story_folder, "ebook", &rows);
    }

    let print_picks = if include_print {
        if include_ebook && same_as_ebook {
            let conn = database.0.lock().unwrap();
            let rows: Vec<(String, String, u8, String)> = ebook_picks
                .iter()
                .map(|(c, h, cf, r)| (c.clone(), h.clone(), *cf, r.clone()))
                .collect();
            let _ = db::replace_bisac_classifications(&conn, story_folder, "print", &rows);
            None
        } else {
            let print_desc = format!("{}\n\n{}", genre_data.industry_print, genre_data.genre_signals);
            let picks = ai_pick_bisac(database, provider, api_key, model, &print_desc, &bisac_master)
                .await
                .unwrap_or_default();
            let conn = database.0.lock().unwrap();
            let rows: Vec<(String, String, u8, String)> = picks
                .iter()
                .map(|(c, h, cf, r)| (c.clone(), h.clone(), *cf, r.clone()))
                .collect();
            let _ = db::replace_bisac_classifications(&conn, story_folder, "print", &rows);
            Some(picks)
        }
    } else {
        None
    };

    if !include_ebook && include_print {
        if let Some(ref picks) = print_picks {
            ebook_picks = picks.clone();
        }
    }

    serde_json::json!({
        "ebook": if include_ebook {
            serde_json::json!(ebook_picks.iter().map(|(code, heading, conf, reason)| serde_json::json!({
                "code": code, "heading": heading, "confidence": conf, "reason": reason,
            })).collect::<Vec<_>>())
        } else {
            serde_json::json!([])
        },
        "print": match &print_picks {
            None if include_print && include_ebook && same_as_ebook => serde_json::json!("same_as_ebook"),
            None => serde_json::json!([]),
            Some(picks) => serde_json::json!(picks.iter().map(|(code, heading, conf, reason)| serde_json::json!({
                "code": code, "heading": heading, "confidence": conf, "reason": reason,
            })).collect::<Vec<_>>()),
        },
    })
    .to_string()
}

/// When two codes are close in fit confidence (within 5 points), prefer a more
/// specific heading over a catch-all "/ General" one. Only re-orders genuinely
/// close calls — a clear fit winner from the AI is never overridden.
pub(crate) fn resort_bisac_for_specificity(mut picks: Vec<(String, String, u8, String)>) -> Vec<(String, String, u8, String)> {
    picks.sort_by(|a, b| {
        let conf_diff = (b.2 as i32 - a.2 as i32).abs();
        if conf_diff <= 5 {
            let specific_a = !a.1.to_lowercase().trim_end().ends_with("general");
            let specific_b = !b.1.to_lowercase().trim_end().ends_with("general");
            if specific_a != specific_b { return specific_b.cmp(&specific_a); }
        }
        b.2.cmp(&a.2)
    });
    picks
}

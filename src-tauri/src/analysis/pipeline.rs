// analysis/pipeline.rs — Orchestration commands that compose the analysis pipeline.

use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

#[derive(Clone, Copy)]
struct PublishFormats {
    ebook: bool,
    print: bool,
}

impl PublishFormats {
    fn from_flags(ebook: bool, print: bool) -> Self {
        if !ebook && !print {
            Self { ebook: true, print: true }
        } else {
            Self { ebook, print }
        }
    }
}

use super::{emit, err, GenreResult, FolderRequest, AnalyzeStoryRequest};
use crate::db;
use crate::models::KeywordResult;

use super::chapters::{collect_chapters, phase1_summaries, phase1_config_from, clean_for_ai, chapter_source_hash, compute_manuscript_fingerprint};
use super::genres::{load_ranked_genres, phase2_analyze, render_full_report};
use super::categories::{match_categories_by_store, rank_by_discoverability};
use super::bisac::run_bisac_classification;
use super::content_advisory::{aggregate_content_signals, generate_content_maturity_advisory};
use super::keywords::{
    call_keyword_optimizer, call_keyword_optimizer_with_pool,
    derive_keyword_seeds, derive_wide_keyword_seeds,
    run_keyword_searches_canopy, run_keyword_searches_dataforseo,
    run_google_keyword_searches_dataforseo,
    generate_discovery_keywords, generate_mi_search_terms, render_kdp_keywords, render_search_terms,
};

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct ReportFreshness {
    pub doc_type: String,
    pub status:   String,
}

#[derive(serde::Serialize)]
pub struct AnalysisState {
    pub has_folder:                 bool,
    pub summary_count:              usize,
    pub summary_chapter_count:      usize,
    pub summary_missing_count:      usize,
    pub summary_stale_count:        usize,
    pub summary_missing_files:      Vec<String>,
    pub summary_stale_files:        Vec<String>,
    pub has_genre_data:             bool,
    pub has_full_report:            bool,
    pub has_wide_analysis:          bool,
    pub has_keywords:               bool,
    pub has_search_terms:           bool,
    pub has_competition:            bool,
    pub has_categories:             bool,
    pub has_genre_ranking:          bool,
    pub has_mapped_verified:        bool,
    pub has_bisac:                  bool,
    pub has_discovery_keywords:     bool,
    pub has_keyword_search_results: bool,
    pub has_google_keyword_search:  bool,
    pub has_zeigarnik:              bool,
    pub has_continuity_check:       bool,
    pub has_show_dont_tell:         bool,
    pub has_ai_isms:                bool,
    /// All doc_types with a current saved document (for generic exists checks).
    pub existing_docs:              Vec<String>,
    pub report_freshness:           Vec<ReportFreshness>,
    pub manuscript_fingerprint:     String,
}

// ── Folder picker ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn pick_manuscript_folder(
    app: AppHandle,
    title: Option<String>,
) -> Result<String, String> {
    use tauri_plugin_dialog::FilePath;
    let dialog_title = title.unwrap_or_else(|| "Select Manuscript Folder".to_string());
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .set_title(dialog_title)
        .pick_folder(move |result| { let _ = tx.send(result); });
    match rx.recv() {
        Ok(Some(FilePath::Path(p))) => Ok(p.to_string_lossy().to_string()),
        Ok(_) => Err("No folder selected".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

// ── Analysis state check ──────────────────────────────────────────────────────

#[tauri::command]
pub async fn check_analysis_state(app: AppHandle, folder: String) -> AnalysisState {
    tokio::task::spawn_blocking(move || {
        let folder_path = PathBuf::from(&folder);
        let current_fp = compute_manuscript_fingerprint(&folder_path);
        let database    = app.state::<db::Db>();
        let conn        = database.0.lock().unwrap();
        let _ = db::sync_manuscript_state(&conn, &folder, &current_fp);

        let chapters    = collect_chapters(&folder_path);
        let summary_hashes = db::load_chapter_summary_hashes(&conn, &folder);

        let mut summary_missing_count = 0usize;
        let mut summary_stale_count = 0usize;
        let mut summary_missing_files: Vec<String> = Vec::new();
        let mut summary_stale_files: Vec<String> = Vec::new();

        for chapter in &chapters {
            let Some(file) = chapter.file_name().map(|f| f.to_string_lossy().to_string()) else {
                continue;
            };

            let Some(stored_hash) = summary_hashes.get(&file) else {
                summary_missing_count += 1;
                summary_missing_files.push(file);
                continue;
            };

            let source = std::fs::read_to_string(chapter).unwrap_or_default();
            let cleaned = clean_for_ai(&source);
            let current_hash = chapter_source_hash(&cleaned);
            if cleaned.is_empty() {
                continue;
            }
            if current_hash != *stored_hash {
                summary_stale_count += 1;
                summary_stale_files.push(file);
            } else if !db::chapter_has_current_summary(&conn, &folder, &file, &current_hash) {
                summary_stale_count += 1;
                summary_stale_files.push(file);
            }
        }

        let report_doc_types = [
            "genre_analysis",
            "genre_ranking",
            "kdp_categories",
            "kdp_keywords",
            "bisac_classification",
            "mi_search_terms",
            "discovery_keywords",
            "google_keyword_search",
            "content_maturity_advisory",
            "wide_metadata_paste",
            "wide_analysis",
            "analysis",
            "keyword_search",
            "competition_report",
            "review_mining",
            "author_analysis",
            "zeigarnik_analysis",
            "continuity_check",
            "show_dont_tell",
            "ai_isms",
        ];
        let report_freshness: Vec<ReportFreshness> = report_doc_types
            .iter()
            .map(|dt| ReportFreshness {
                doc_type: dt.to_string(),
                status: db::report_freshness_status(&conn, &folder, dt, &current_fp).as_str().to_string(),
            })
            .collect();

        let genre_fresh = db::artifact_status(&conn, &folder, "genre_data", &current_fp) == db::Freshness::Fresh;

        AnalysisState {
            has_folder:                 folder_path.exists(),
            summary_count:              db::chapter_summary_count(&conn, &folder) as usize,
            summary_chapter_count:      chapters.len(),
            summary_missing_count,
            summary_stale_count,
            summary_missing_files,
            summary_stale_files,
        has_genre_data:             db::load_genre_data(&conn, &folder).is_some() && genre_fresh,
        has_full_report:            db::report_freshness_status(&conn, &folder, "analysis", &current_fp) == db::Freshness::Fresh,
        has_wide_analysis:          db::report_freshness_status(&conn, &folder, "wide_analysis", &current_fp) == db::Freshness::Fresh,
        has_keywords:               db::load_kdp_keywords(&conn, &folder).is_some()
            && db::report_freshness_status(&conn, &folder, "kdp_keywords", &current_fp) == db::Freshness::Fresh,
        has_search_terms:           !db::load_mi_search_terms(&conn, &folder).is_empty()
            && db::report_freshness_status(&conn, &folder, "mi_search_terms", &current_fp) == db::Freshness::Fresh,
        has_competition:            db::report_freshness_status(&conn, &folder, "competition_report", &current_fp) == db::Freshness::Fresh,
        has_categories:             db::has_category_results(&conn, &folder)
            && db::artifact_status(&conn, &folder, "categories", &current_fp) == db::Freshness::Fresh,
        has_genre_ranking:          db::has_genre_rankings(&conn, &folder) && genre_fresh,
        has_mapped_verified:        db::report_freshness_status(&conn, &folder, "mapped_categories", &current_fp) == db::Freshness::Fresh,
        has_bisac:                  db::has_bisac_classifications(&conn, &folder)
            && db::artifact_status(&conn, &folder, "bisac", &current_fp) == db::Freshness::Fresh,
        has_discovery_keywords:     !db::load_discovery_keywords(&conn, &folder).is_empty()
            && db::report_freshness_status(&conn, &folder, "discovery_keywords", &current_fp) == db::Freshness::Fresh,
        has_keyword_search_results: db::has_keyword_search_results(&conn, &folder)
            && db::artifact_status(&conn, &folder, "keyword_search", &current_fp) == db::Freshness::Fresh,
        has_google_keyword_search: db::has_google_keyword_search_results(&conn, &folder)
            && db::artifact_status(&conn, &folder, "google_keyword_search", &current_fp) == db::Freshness::Fresh,
        has_zeigarnik:              db::has_zeigarnik_analysis(&conn, &folder)
            && db::artifact_status(&conn, &folder, "zeigarnik", &current_fp) == db::Freshness::Fresh,
            has_continuity_check:       db::report_freshness_status(&conn, &folder, "continuity_check", &current_fp) == db::Freshness::Fresh,
            has_show_dont_tell:         db::report_freshness_status(&conn, &folder, "show_dont_tell", &current_fp) == db::Freshness::Fresh,
            has_ai_isms:                db::report_freshness_status(&conn, &folder, "ai_isms", &current_fp) == db::Freshness::Fresh,
            existing_docs:              db::list_existing_doc_types(&conn, &folder),
            report_freshness,
            manuscript_fingerprint:     current_fp,
        }
    }).await.unwrap()
}

// ── run_everything ────────────────────────────────────────────────────────────

/// Run everything except folder selection and chapter summaries:
/// Analyze Genre → KDP Analysis (categories + keywords) → Generate Search Terms
#[tauri::command]
pub async fn run_everything(app: AppHandle, request: FolderRequest) -> GenreResult {
    let folder = PathBuf::from(&request.folder);
    if !folder.exists() { return err("Folder does not exist."); }

    crate::reset_cancel();
    let database = app.state::<db::Db>();
    let run_ts = chrono::Utc::now().to_rfc3339();

    // ── Step 1: Ensure summaries exist ────────────────────────────────────
    let mut summaries = { let conn = database.0.lock().unwrap(); db::load_chapter_summaries(&conn, &request.folder) };
    if summaries.is_empty() {
        emit(&app, "Step 1: No summaries found — generating now...");
        let chapters = collect_chapters(&folder);
        if chapters.is_empty() { return err("No .md chapter files found."); }
        phase1_summaries(
            &app,
            &database,
            &chapters,
            &request.folder,
            &phase1_config_from(
                &request.provider,
                &request.api_key,
                &request.model,
                &request.summaries_model,
                false,
            ),
        )
        .await;
        let conn = database.0.lock().unwrap();
        summaries = db::load_chapter_summaries(&conn, &request.folder);
        if summaries.is_empty() { return err("Could not produce chapter summaries."); }
    } else {
        emit(&app, &format!("Step 1: {} summaries found — skipping.", summaries.len()));
    }
    if crate::is_cancelled() { return err("Cancelled."); }

    // ── Step 2: Genre analysis ─────────────────────────────────────────────
    emit(&app, "Step 2: Running genre analysis...");
    let genre_result = phase2_analyze(
        &app, &database, &request.folder, &summaries,
        &request.provider, &request.api_key, &request.model,
        &request.genre_model,
    ).await;
    if !genre_result.success { return genre_result; }
    if crate::is_cancelled() { return err("Cancelled."); }

    // ── Step 3: Full report ────────────────────────────────────────────────
    emit(&app, "Step 3: Building full report...");
    let genre_data = { let conn = database.0.lock().unwrap(); db::load_genre_data(&conn, &request.folder) };
    let genre_data = match genre_data {
        Some(d) => d,
        None    => return err("genre_data missing after analysis."),
    };
    let ranked = load_ranked_genres(&database, &request.folder);
    let full_report = render_full_report(&genre_data, &ranked, false);
    { let conn = database.0.lock().unwrap(); let _ = db::save_document_at(&conn, &request.folder, "full_report", &full_report, &run_ts); }
    emit(&app, "  ✓ Full report saved to database.");
    if crate::is_cancelled() { return err("Cancelled."); }

    // ── Step 4: Optimize KDP keywords ─────────────────────────────────────
    emit(&app, "Step 4: Optimizing KDP keywords...");
    match call_keyword_optimizer(&database, &request.provider, &request.api_key, &request.model, &genre_data, &genre_data.genre_signals).await {
        Ok((entries, strategy)) => {
            let conn = database.0.lock().unwrap();
            let _ = db::save_kdp_keywords(&conn, &request.folder, &entries, &strategy, "*(Generated from genre analysis.)*");
            let rendered = render_kdp_keywords(&entries, &strategy, "*(Generated from genre analysis.)*");
            let _ = db::save_document_at(&conn, &request.folder, "kdp_keywords", &rendered, &run_ts);
            emit(&app, "  ✓ KDP keywords saved to database.");
        }
        Err(e) => emit(&app, &format!("  ⚠ Keyword optimization failed: {}", e)),
    }
    if crate::is_cancelled() { return err("Cancelled."); }

    // ── Step 5: Generate search terms ──────────────────────────────────────
    emit(&app, "Step 5: Generating competition search terms...");
    match generate_mi_search_terms(&database, &request.provider, &request.api_key, &request.model, &genre_data).await {
        Ok(keywords) => {
            let conn = database.0.lock().unwrap();
            let _ = db::save_mi_search_terms(&conn, &request.folder, &keywords);
            let rendered = render_search_terms(&keywords);
            let _ = db::save_document_at(&conn, &request.folder, "mi_search_terms", &rendered, &run_ts);
            emit(&app, &format!("  ✓ {} search terms saved to database.", keywords.len()));
            for kw in &keywords { emit(&app, &format!("    • {}", kw)); }
        }
        Err(e) => emit(&app, &format!("  ⚠ Search terms generation failed: {}", e)),
    }

    emit(&app, "✓ Analysis complete. Run Analyze Competition next.");

    GenreResult { success: true, report: full_report, error: String::new(), run_ts: run_ts.clone() }
}

// ── run_full_analysis ─────────────────────────────────────────────────────────

#[tauri::command]
pub async fn run_full_analysis(app: AppHandle, request: FolderRequest) -> GenreResult {
    let folder = PathBuf::from(&request.folder);
    if !folder.exists() { return err("Folder does not exist."); }

    let database = app.state::<db::Db>();
    let run_ts = chrono::Utc::now().to_rfc3339();

    // ── Phase 1 ──────────────────────────────────────────────────────────
    let mut summaries = { let conn = database.0.lock().unwrap(); db::load_chapter_summaries(&conn, &request.folder) };
    if summaries.is_empty() {
        emit(&app, "Phase 1: Generating chapter summaries...");
        let chapters = collect_chapters(&folder);
        if chapters.is_empty() { return err("No .md files found."); }
        phase1_summaries(
            &app,
            &database,
            &chapters,
            &request.folder,
            &phase1_config_from(
                &request.provider,
                &request.api_key,
                &request.model,
                &request.summaries_model,
                false,
            ),
        )
        .await;
        let conn = database.0.lock().unwrap();
        summaries = db::load_chapter_summaries(&conn, &request.folder);
    } else {
        emit(&app, &format!("Phase 1: {} summaries already exist — skipping.", summaries.len()));
    }
    if summaries.is_empty() { return err("No chapter summaries available."); }

    // ── Phase 2 ──────────────────────────────────────────────────────────
    let existing = { let conn = database.0.lock().unwrap(); db::load_genre_data(&conn, &request.folder) };
    let genre_data = if let Some(d) = existing {
        emit(&app, "Phase 2: genre data exists in database — loading...");
        d
    } else {
        emit(&app, "Phase 2: Running genre analysis...");
        let r = phase2_analyze(
        &app, &database, &request.folder, &summaries,
        &request.provider, &request.api_key, &request.model,
        &request.genre_model,
    ).await;
        if !r.success { return r; }
        let conn = database.0.lock().unwrap();
        match db::load_genre_data(&conn, &request.folder) {
            Some(d) => d,
            None    => return err("Phase 2 produced no genre data."),
        }
    };

    emit(&app, &format!("  KDP ebook paths: {}", genre_data.kdp_ebook.join(", ")));
    emit(&app, &format!("  KDP print paths: {}", genre_data.kdp_print.join(", ")));

    // ── Build full report ─────────────────────────────────────────────────
    emit(&app, "Building full report...");
    let full_report = render_full_report(&genre_data, &load_ranked_genres(&database, &request.folder), true);
    { let conn = database.0.lock().unwrap(); let _ = db::save_document_at(&conn, &request.folder, "full_report", &full_report, &run_ts); }
    emit(&app, "✓ Full report saved to database.");

    GenreResult { success: true, report: full_report, error: String::new(), run_ts: run_ts.clone() }
}

// ── find_genres_and_categories_for_story ──────────────────────────────────────

#[tauri::command]
pub async fn find_genres_and_categories_for_story(app: AppHandle, request: FolderRequest) -> GenreResult {
    let cancel = crate::cancel_notify();
    tokio::select! {
        result = find_genres_and_categories_inner(app, request) => result,
        _ = cancel.notified() => err("Cancelled."),
    }
}

async fn find_genres_and_categories_inner(app: AppHandle, request: FolderRequest) -> GenreResult {
    let database = app.state::<db::Db>();
    let run_ts = chrono::Utc::now().to_rfc3339();

    // ── Ensure genre_data exists ──
    let mut genre_data = { let conn = database.0.lock().unwrap(); db::load_genre_data(&conn, &request.folder) };
    if genre_data.is_none() {
        emit(&app, "No genre data yet — running Analyze first...");
        let folder_path = PathBuf::from(&request.folder);
        if !folder_path.exists() { return err("Folder does not exist."); }

        let mut summaries = { let conn = database.0.lock().unwrap(); db::load_chapter_summaries(&conn, &request.folder) };
        if summaries.is_empty() {
            let chapters = collect_chapters(&folder_path);
            if chapters.is_empty() { return err("No .md chapter files found."); }
            phase1_summaries(
            &app,
            &database,
            &chapters,
            &request.folder,
            &phase1_config_from(
                &request.provider,
                &request.api_key,
                &request.model,
                &request.summaries_model,
                false,
            ),
        )
        .await;
            let conn = database.0.lock().unwrap();
            summaries = db::load_chapter_summaries(&conn, &request.folder);
        }
        if summaries.is_empty() { return err("Could not produce chapter summaries."); }

        let r = phase2_analyze(
        &app, &database, &request.folder, &summaries,
        &request.provider, &request.api_key, &request.model,
        &request.genre_model,
    ).await;
        if !r.success { return err(&r.error); }
        genre_data = { let conn = database.0.lock().unwrap(); db::load_genre_data(&conn, &request.folder) };
    }
    let genre_data = match genre_data {
        Some(d) => d,
        None    => return err("Could not produce genre data."),
    };

    let mut ranked = load_ranked_genres(&database, &request.folder);
    if ranked.is_empty() {
        emit(&app, "Ranking manuscript against master genre list...");
        ranked = match super::genres::run_and_persist_genre_ranking(
            &app,
            &database,
            &request.folder,
            &genre_data,
            &request.provider,
            &request.api_key,
            &request.model,
            &request.genre_model,
        )
        .await
        {
            Ok(r) => r,
            Err(e) => return err(&format!("Genre ranking failed: {}", e)),
        };
    }

    let mut report_sections: Vec<String> = Vec::new();

    report_sections.push({
        let mut s = vec!["## Genre Ranking".to_string(), String::new(),
            "Scored independently — percentages do not sum to 100.".to_string(), String::new()];
        for r in &ranked { s.push(format!("- **{}** — {}%", r.genre, r.confidence)); }
        s.push(String::new());
        s.join("\n")
    });

    let genre_terms: Vec<(String, u8)> = if !ranked.is_empty() {
        ranked.iter().filter(|r| r.confidence >= 30).take(6).map(|r| (r.genre.clone(), r.confidence)).collect()
    } else {
        vec![(genre_data.industry_ebook.clone(), 100)]
    };

    // ── KDP Categories, both formats ──
    emit(&app, "Matching KDP categories against the imported catalog...");
    let base_description = format!("{}\n\n{}", genre_data.industry_ebook, genre_data.genre_signals);
    let mut kdp_section = vec!["## KDP Categories".to_string(), String::new()];
    for (store, label) in [("Kindle", "Kindle eBook"), ("Books", "Paperback")] {
        kdp_section.push(format!("### {}", label));
        kdp_section.push(String::new());
        let total_catalog = { let conn = database.0.lock().unwrap(); db::kdp_category_count(&conn, store) };
        if total_catalog < 50 {
            kdp_section.push("*Catalog nearly empty for this store — import WinningCat data, or use Find Categories (PR).*".to_string());
            kdp_section.push(String::new());
            continue;
        }

        let result = match_categories_by_store(&app, &database, &request.folder, store, &base_description, &genre_terms, &request.provider, &request.api_key, &request.model).await;

        let final_cats = rank_by_discoverability(&app, store, result.qualifying, &request.canopy_api_key).await;

        if final_cats.is_empty() {
            kdp_section.push("*No candidates cleared the fit bar for this store.*".to_string());
        } else {
            for (i, q) in final_cats.iter().enumerate() {
                let bonus = if i >= 3 { " — bonus candidate for post-launch" } else { "" };
                let disc_note = if q.verified {
                    format!(" — sales to #10: {}", q.sales_to_ten)
                } else {
                    " — could not verify live".to_string()
                };
                kdp_section.push(format!("{}. `{}` (fit {}%){}{} — matched by: {}", i + 1, q.path, q.fit_confidence, bonus, disc_note, q.agreeing_genres.join(", ")));
                if !q.top_books.is_empty() {
                    kdp_section.push(String::new());
                    kdp_section.push("   **Current Top Sellers:**".to_string());
                    for (rank, book) in q.top_books.iter().enumerate() {
                        let amazon_link = format!("https://www.amazon.com/dp/{}", book.asin);
                        let img_tag = book.image_url.as_deref()
                            .map(|url| format!("   <img src=\"{}\" height=\"60\" /> ", url))
                            .unwrap_or_default();
                        kdp_section.push(format!("   {}{}. [{}]({})", img_tag, rank + 1, book.title, amazon_link));
                    }
                    kdp_section.push(String::new());
                }
            }
        }
        kdp_section.push(String::new());
    }
    report_sections.push(kdp_section.join("\n"));

    // ── BISAC, ebook then print if different ───────────────────────
    emit(&app, "Classifying BISAC subject headings...");
    let bisac_json = run_bisac_classification(
        &database, &request.folder, &genre_data,
        &request.provider, &request.api_key, &request.model,
        true, true,
    ).await;
    let bisac_value: serde_json::Value = serde_json::from_str(&bisac_json).unwrap_or(serde_json::json!({}));
    let ebook_picks: Vec<(String, String, u8, String)> = bisac_value["ebook"].as_array()
        .map(|arr| arr.iter().filter_map(|v| {
            Some((v["code"].as_str()?.to_string(), v["heading"].as_str()?.to_string(),
                v["confidence"].as_u64()? as u8, v["reason"].as_str().unwrap_or("").to_string()))
        }).collect())
        .unwrap_or_default();
    let print_picks_opt: Option<Vec<(String, String, u8, String)>> = if bisac_value["print"].as_str() == Some("same_as_ebook") {
        None
    } else {
        bisac_value["print"].as_array().map(|arr| arr.iter().filter_map(|v| {
            Some((v["code"].as_str()?.to_string(), v["heading"].as_str()?.to_string(),
                v["confidence"].as_u64()? as u8, v["reason"].as_str().unwrap_or("").to_string()))
        }).collect())
    };

    let mut bisac_section = vec![
        "## BISAC Classification".to_string(), String::new(),
        "*Verify against BISG's free lookup (bisg.org/complete-bisac-subject-headings-list) before submitting anywhere. Kindle eBook no longer takes BISAC directly on KDP; this matters for KDP Print and wide/Ingram distribution. No live discoverability data exists for BISAC — close calls are broken by preferring a specific heading over a generic \"/ General\" one, a structural heuristic, not measured data.*".to_string(),
        String::new(),
    ];

    bisac_section.push("### Ebook".to_string());
    bisac_section.push(String::new());
    if ebook_picks.is_empty() {
        bisac_section.push("*No confident BISAC match.*".to_string());
    } else {
        for (i, (code, heading, conf, _reason)) in ebook_picks.iter().enumerate() {
            bisac_section.push(format!("{}. `{}` — {} ({}%)", i + 1, code, heading, conf));
        }
    }
    bisac_section.push(String::new());

    bisac_section.push("### Print".to_string());
    bisac_section.push(String::new());
    match &print_picks_opt {
        None => bisac_section.push("*Same as ebook — print genre tag matches ebook.*".to_string()),
        Some(print_picks) => {
            let ebook_codes: std::collections::HashSet<String> = ebook_picks.iter().map(|(c, _, _, _)| c.clone()).collect();
            let print_codes: std::collections::HashSet<String> = print_picks.iter().map(|(c, _, _, _)| c.clone()).collect();
            if !print_picks.is_empty() && ebook_codes == print_codes {
                bisac_section.push("*Same codes as ebook.*".to_string());
            } else if print_picks.is_empty() {
                bisac_section.push("*No confident BISAC match.*".to_string());
            } else {
                for (i, (code, heading, conf, _reason)) in print_picks.iter().enumerate() {
                    bisac_section.push(format!("{}. `{}` — {} ({}%)", i + 1, code, heading, conf));
                }
            }
        }
    }
    bisac_section.push(String::new());
    report_sections.push(bisac_section.join("\n"));

    // ── Positioning context ──
    let mut context_section = vec!["## Positioning Context".to_string(), String::new()];
    context_section.push(format!("**Reader demographic:** {}", genre_data.reader_demographic));
    context_section.push(format!("**Bookstore shelving:** {}", genre_data.bookstore_shelving));
    if !genre_data.comps_ebook.is_empty() {
        context_section.push(String::new());
        context_section.push("**Ebook comps:**".to_string());
        for c in &genre_data.comps_ebook { context_section.push(format!("- {}", c)); }
    }
    if !genre_data.comps_print.is_empty() {
        context_section.push(String::new());
        context_section.push("**Print comps:**".to_string());
        for c in &genre_data.comps_print { context_section.push(format!("- {}", c)); }
    }
    context_section.push(String::new());
    report_sections.push(context_section.join("\n"));

    let now = chrono::Utc::now().format("%B %-d, %Y %H:%M UTC").to_string();
    let mut lines = vec![
        "# Find Genres & Categories".to_string(),
        format!("Generated: {}", now),
        "Full pipeline in one pass: genre ranking, KDP categories (Kindle eBook + Paperback, verified live via Canopy API), BISAC classification (ebook + print), and positioning context.".to_string(),
        String::new(), "---".to_string(), String::new(),
    ];
    lines.push(report_sections.join("\n---\n\n"));
    let report = lines.join("\n");

    { let conn = database.0.lock().unwrap(); let _ = db::save_document_at(&conn, &request.folder, "genres_and_categories", &report, &run_ts); }
    emit(&app, "✓ Genres & Categories report saved to database.");

    GenreResult { success: true, report, error: String::new(), run_ts: run_ts.clone() }
}

// ── Combined Report Assembly ──────────────────────────────────────────────────

/// Assembles all pipeline output sections into a single structured JSON document.
pub(crate) fn render_combined_report(
    kdp_paste_section: &str,
    genre_ranking_section: &str,
    kdp_categories_section: &str,
    bisac_section: &str,
    kdp_keywords_section: &str,
    discovery_keywords_section: &str,
    positioning_section: &str,
    description_snippet: Option<&str>,
) -> String {
    let json = serde_json::json!({
        "schema": "analysis_v1",
        "sections": {
            "kdp_paste": kdp_paste_section,
            "genre_ranking": genre_ranking_section,
            "kdp_categories": kdp_categories_section,
            "bisac": bisac_section,
            "kdp_keywords": kdp_keywords_section,
            "discovery_keywords": discovery_keywords_section,
            "positioning": positioning_section,
            "description": description_snippet.unwrap_or(""),
        }
    });
    json.to_string()
}

/// Assembles wide-distribution outputs into one saved report.
pub(crate) fn render_wide_combined_report(
    genre_ranking_section: &str,
    bisac_section: &str,
    discovery_keywords_section: &str,
    google_keywords_section: &str,
    content_advisory_section: &str,
    wide_paste_section: &str,
    positioning_section: &str,
) -> String {
    serde_json::json!({
        "schema": "wide_analysis_v1",
        "sections": {
            "genre_ranking": genre_ranking_section,
            "bisac": bisac_section,
            "discovery_keywords": discovery_keywords_section,
            "google_keywords": google_keywords_section,
            "content_advisory": content_advisory_section,
            "wide_paste": wide_paste_section,
            "positioning": positioning_section,
        }
    }).to_string()
}

// ── analyze_story ─────────────────────────────────────────────────────────────

/// Whether a publish-platform report may run on KDP or Wide.
fn report_allowed_on_platform(report_id: &str, platform: &str) -> bool {
    match report_id {
        "chapter_summaries" | "genre_analysis" | "genre_ranking" => {
            platform == "kdp" || platform == "wide"
        }
        "kdp_categories" | "kdp_keywords" | "mi_search_terms" | "keyword_search" | "analysis"
        | "competition_report" | "review_mining" | "author_analysis" | "wide_analysis" => {
            platform == "kdp"
        }
        "bisac_classification" | "discovery_keywords" | "google_keyword_search"
        | "content_maturity_advisory" | "wide_metadata_paste" => platform == "kdp",
        _ => false,
    }
}

/// True when `report_id` is selected and allowed on the active platform.
fn wants_report(selected: &[String], report_id: &str, platform: &str) -> bool {
    selected.iter().any(|s| s == report_id) && report_allowed_on_platform(report_id, platform)
}

/// KDP Analysis bundles genre classification, category matching, and keyword optimization.
fn wants_kdp_analysis_bundle(selected: &[String], platform: &str) -> bool {
    wants_report(selected, "analysis", platform)
}

/// Wide Analysis bundles BISAC, discovery keywords, content advisory, and paste sheet.
fn wants_wide_analysis_bundle(selected: &[String], platform: &str) -> bool {
    wants_report(selected, "wide_analysis", platform)
}

fn should_run_genre_analysis(selected: &[String], platform: &str) -> bool {
    wants_report(selected, "genre_analysis", platform)
        || wants_kdp_analysis_bundle(selected, platform)
        || wants_wide_analysis_bundle(selected, platform)
        || wants_report(selected, "bisac_classification", platform)
        || wants_report(selected, "discovery_keywords", platform)
        || wants_report(selected, "google_keyword_search", platform)
        || wants_report(selected, "content_maturity_advisory", platform)
        || wants_report(selected, "wide_metadata_paste", platform)
}

fn should_run_bisac(selected: &[String], platform: &str, formats: PublishFormats) -> bool {
    if wants_report(selected, "bisac_classification", platform) || wants_wide_analysis_bundle(selected, platform) {
        return formats.ebook || formats.print;
    }
    wants_kdp_analysis_bundle(selected, platform) && formats.print
}

fn should_run_discovery_keywords(selected: &[String], platform: &str, formats: PublishFormats) -> bool {
    (wants_report(selected, "discovery_keywords", platform) || wants_wide_analysis_bundle(selected, platform))
        && formats.ebook
}

fn should_run_content_advisory(selected: &[String], platform: &str, formats: PublishFormats) -> bool {
    (wants_report(selected, "content_maturity_advisory", platform) || wants_wide_analysis_bundle(selected, platform))
        && formats.ebook
}

fn should_run_google_keyword_search(selected: &[String], platform: &str, formats: PublishFormats) -> bool {
    (wants_report(selected, "google_keyword_search", platform) || wants_wide_analysis_bundle(selected, platform))
        && formats.ebook
}

fn should_run_wide_paste(selected: &[String], platform: &str) -> bool {
    wants_report(selected, "wide_metadata_paste", platform) || wants_wide_analysis_bundle(selected, platform)
}

fn should_run_kdp_categories(selected: &[String], platform: &str, formats: PublishFormats) -> bool {
    if !(wants_report(selected, "kdp_categories", platform) || wants_kdp_analysis_bundle(selected, platform)) {
        return false;
    }
    formats.ebook || formats.print
}

fn should_run_kdp_keywords(selected: &[String], platform: &str) -> bool {
    wants_report(selected, "kdp_keywords", platform) || wants_kdp_analysis_bundle(selected, platform)
}

/// Persist a report the user explicitly selected — always overwrite prior output.
fn save_selected_report(
    conn: &rusqlite::Connection,
    story_folder: &str,
    doc_type: &str,
    content: &str,
    timestamp: &str,
    manuscript_fp: &str,
) {
    let _ = db::save_document_current(conn, story_folder, doc_type, content, timestamp, manuscript_fp);
}

#[tauri::command]
pub async fn analyze_story(app: AppHandle, request: AnalyzeStoryRequest) -> GenreResult {
    let cancel = crate::cancel_notify();
    tokio::select! {
        result = analyze_story_inner(app, request) => result,
        _ = cancel.notified() => err("Cancelled."),
    }
}

async fn analyze_story_inner(app: AppHandle, request: AnalyzeStoryRequest) -> GenreResult {
    let selected = &request.selected;
    let platform = request.platform.as_str();
    let formats = PublishFormats::from_flags(request.publish_ebook, request.publish_print);

    if platform != "kdp" {
        return err("Invalid platform — expected kdp.");
    }
    if selected.is_empty() {
        return err("No reports selected.");
    }

    if let Err(msg) = crate::ai::ai_ready(&request.provider, &request.api_key, &request.model) {
        return err(&msg);
    }

    let database = app.state::<db::Db>();
    let run_ts = if request.run_time.is_empty() { chrono::Utc::now().to_rfc3339() } else { request.run_time.clone() };
    let folder_path = PathBuf::from(&request.folder);
    if !folder_path.exists() { return err("Folder does not exist."); }
    let manuscript_fp = compute_manuscript_fingerprint(&folder_path);
    {
        let conn = database.0.lock().unwrap();
        let _ = db::sync_manuscript_state(&conn, &request.folder, &manuscript_fp);
    }

    let needs_genre_data = should_run_genre_analysis(selected, platform)
        || should_run_kdp_categories(selected, platform, formats)
        || should_run_kdp_keywords(selected, platform)
        || should_run_bisac(selected, platform, formats)
        || wants_report(selected, "mi_search_terms", platform)
        || should_run_discovery_keywords(selected, platform, formats)
        || wants_report(selected, "keyword_search", platform)
        || should_run_google_keyword_search(selected, platform, formats)
        || should_run_content_advisory(selected, platform, formats)
        || wants_kdp_analysis_bundle(selected, platform)
        || wants_wide_analysis_bundle(selected, platform)
        || wants_report(selected, "competition_report", platform)
        || wants_report(selected, "review_mining", platform)
        || wants_report(selected, "author_analysis", platform);

    // ── Step 1: Chapter summaries (infrastructure) ────────────────────────
    if needs_genre_data || request.force_resummarize {
        let summary_status = {
            let conn = database.0.lock().unwrap();
            db::artifact_status(&conn, &request.folder, "summaries", &manuscript_fp)
        };
        let phase1_config = phase1_config_from(
            &request.provider,
            &request.api_key,
            &request.model,
            &request.summaries_model,
            request.force_resummarize,
        );
        if request.force_resummarize {
            emit(&app, "  Force re-summarize — clearing existing chapter summaries...");
            let conn = database.0.lock().unwrap();
            let _ = db::delete_chapter_summaries(&conn, &request.folder);
        }
        let chapters = collect_chapters(&folder_path);
        let summaries_stale = chapters.iter().any(|chapter| {
            let Some(file) = chapter.file_name().map(|f| f.to_string_lossy().to_string()) else {
                return false;
            };
            let source = std::fs::read_to_string(chapter).unwrap_or_default();
            let cleaned = clean_for_ai(&source);
            if cleaned.is_empty() {
                return false;
            }
            let hash = chapter_source_hash(&cleaned);
            let conn = database.0.lock().unwrap();
            !db::chapter_has_current_summary(&conn, &request.folder, &file, &hash)
        });
        if summary_status != db::Freshness::Fresh || request.force_resummarize || summaries_stale {
            emit(&app, "Step 1: Summarizing chapters (AI)...");
            crate::reset_cancel();
            if chapters.is_empty() {
                return err("No .md chapter files found.");
            }
            let (done, skipped) =
                phase1_summaries(&app, &database, &chapters, &request.folder, &phase1_config).await;
            emit(&app, &format!("  ✓ {} summarized, {} skipped.", done, skipped));
            {
                let conn = database.0.lock().unwrap();
                let _ = db::record_artifact_built(&conn, &request.folder, "summaries", &manuscript_fp);
            }
        } else {
            emit(&app, "Step 1: Chapter summaries up to date — skipping.");
        }
        if crate::is_cancelled() {
            return err("Cancelled.");
        }
    }

    // ── Step 2: Genre Analysis ─────────────────────────────────────────────
    if should_run_genre_analysis(selected, platform) {
        emit(&app, "Step 2: Genre analysis...");
        let summaries = { let conn = database.0.lock().unwrap(); db::load_chapter_summaries(&conn, &request.folder) };
        if summaries.is_empty() { return err("No chapter summaries available."); }
        let r = phase2_analyze(
            &app, &database, &request.folder, &summaries,
            &request.provider, &request.api_key, &request.model,
            &request.genre_model,
        ).await;
        if !r.success { return err(&r.error); }
        {
            let conn = database.0.lock().unwrap();
            let _ = db::record_artifact_built(&conn, &request.folder, "genre_data", &manuscript_fp);
            let _ = db::record_artifact_built(&conn, &request.folder, "genre_ranking", &manuscript_fp);
        }
        if crate::is_cancelled() { return err("Cancelled."); }
    }

    let genre_data = if needs_genre_data {
        let genre_data = { let conn = database.0.lock().unwrap(); db::load_genre_data(&conn, &request.folder) };
        match genre_data {
            Some(d) => d,
            None => return err("Genre analysis data is required. Run a positioning report first (KDP Analysis or Wide Analysis)."),
        }
    } else {
        return GenreResult {
            success: true,
            report: String::new(),
            error: String::new(),
            run_ts: run_ts.clone(),
        };
    };

    let ranked = load_ranked_genres(&database, &request.folder);
    let genre_ranking_section = if ranked.is_empty() {
        String::new()
    } else {
        serde_json::json!({
            "genres": ranked.iter().map(|r| serde_json::json!({
                "genre": r.genre,
                "confidence": r.confidence,
                "reason": r.reason,
            })).collect::<Vec<_>>(),
        })
        .to_string()
    };

    let genre_terms: Vec<(String, u8)> = if !ranked.is_empty() {
        ranked.iter().filter(|r| r.confidence >= 30).take(6).map(|r| (r.genre.clone(), r.confidence)).collect()
    } else {
        vec![(genre_data.industry_ebook.clone(), 100)]
    };

    // ── Step 4: KDP Categories (both stores) ───────────────────────────────
    let mut kindle_top_categories: Vec<String> = Vec::new();
    let mut print_top_categories: Vec<String> = Vec::new();
    let mut kdp_categories_section = serde_json::json!({ "stores": [] }).to_string();
    if should_run_kdp_categories(selected, platform, formats) {
        emit(&app, "Step 4: Matching KDP categories...");
        let base_description = format!("{}\n\n{}", genre_data.industry_ebook, genre_data.genre_signals);
        let mut kdp_stores_json: Vec<serde_json::Value> = Vec::new();

        let store_jobs: Vec<(&str, &str)> = [
            formats.ebook.then_some(("Kindle", "Kindle eBook")),
            formats.print.then_some(("Books", "Paperback")),
        ]
        .into_iter()
        .flatten()
        .collect();

        for (store, label) in store_jobs {
            let top_cats = if store == "Kindle" { &mut kindle_top_categories } else { &mut print_top_categories };
            let total_catalog = { let conn = database.0.lock().unwrap(); db::kdp_category_count(&conn, store) };
            if total_catalog < 50 {
                kdp_stores_json.push(serde_json::json!({ "store": label, "error": "Catalog nearly empty — import WinningCat data." }));
                continue;
            }

            let result = match_categories_by_store(&app, &database, &request.folder, store, &base_description, &genre_terms, &request.provider, &request.api_key, &request.model).await;

            let final_cats = rank_by_discoverability(&app, store, result.qualifying, &request.canopy_api_key).await;

            for q in final_cats.iter().take(3) {
                top_cats.push(q.path.clone());
            }

            kdp_stores_json.push(serde_json::json!({
                "store": label,
                "categories": final_cats.iter().enumerate().map(|(i, q)| serde_json::json!({
                    "rank": i + 1,
                    "path": q.path,
                    "fit_confidence": q.fit_confidence,
                    "sales_to_ten": q.sales_to_ten,
                    "verified": q.verified,
                    "is_bonus": i >= 3,
                    "agreeing_genres": q.agreeing_genres,
                    "top_books": q.top_books.iter().map(|b| serde_json::json!({
                        "title": b.title,
                        "asin": b.asin,
                        "image_url": b.image_url,
                    })).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            }));

            if crate::is_cancelled() { return err("Cancelled."); }
        }
        kdp_categories_section = serde_json::json!({ "stores": kdp_stores_json }).to_string();
        if crate::is_cancelled() { return err("Cancelled."); }
    }

    // ── Step 5: Generate search terms (KDP only) ───────────────────────────
    if wants_report(selected, "mi_search_terms", platform) {
        emit(&app, "Step 5: Generating competition search terms...");
        match generate_mi_search_terms(&database, &request.provider, &request.api_key, &request.model, &genre_data).await {
            Ok(keywords) => {
                let conn = database.0.lock().unwrap();
                let _ = db::save_mi_search_terms(&conn, &request.folder, &keywords);
                let rendered = render_search_terms(&keywords);
                save_selected_report(&conn, &request.folder, "mi_search_terms", &rendered, &run_ts, &manuscript_fp);
                let _ = db::record_artifact_built(&conn, &request.folder, "mi_search_terms", &manuscript_fp);
                emit(&app, &format!("  ✓ {} search terms saved.", keywords.len()));
            }
            Err(e) => emit(&app, &format!("  ⚠ Search terms generation failed: {}", e)),
        }
        if crate::is_cancelled() { return err("Cancelled."); }
    }

    // ── Step 6: BISAC Classification ───────────────────────────────────────
    let mut bisac_section = String::new();
    if should_run_bisac(selected, platform, formats) {
        emit(&app, "Step 6: BISAC classification...");
        bisac_section = run_bisac_classification(
            &database,
            &request.folder,
            &genre_data,
            &request.provider,
            &request.api_key,
            &request.model,
            formats.ebook && wants_wide_analysis_bundle(selected, platform),
            formats.print,
        ).await;
        if !bisac_section.is_empty() {
            let conn = database.0.lock().unwrap();
            save_selected_report(&conn, &request.folder, "bisac_classification", &bisac_section, &run_ts, &manuscript_fp);
            let _ = db::record_artifact_built(&conn, &request.folder, "bisac", &manuscript_fp);
        }
        if crate::is_cancelled() { return err("Cancelled."); }
    }

    // ── Step 7: Keyword Search (KDP only) ────────────────────────────────────
    let mut keyword_pool: Vec<KeywordResult> = Vec::new();
    if wants_report(selected, "keyword_search", platform) {
        emit(&app, "Step 7: Keyword search...");
        let top_cats_for_seeds: Vec<String> = kindle_top_categories.iter().take(2).cloned().collect();
        keyword_pool = {
            let seeds = derive_keyword_seeds(&genre_data.industry_ebook, &top_cats_for_seeds);
            if seeds.is_empty() {
                emit(&app, "  ⚠ No seeds derived — skipping keyword search.");
                Vec::new()
            } else {
                emit(&app, &format!("  Seeds: {:?}", seeds));
                if !request.dataforseo_login.is_empty() && !request.dataforseo_password.is_empty() {
                    run_keyword_searches_dataforseo(&app, &request.folder, &seeds, &request.dataforseo_login, &request.dataforseo_password).await
                } else if !request.canopy_api_key.is_empty() {
                    emit(&app, "⚠ DataForSEO credentials not set — falling back to Canopy for keyword search. Add DataForSEO login/password in Settings for real Amazon search volume data.");
                    run_keyword_searches_canopy(&app, &request.folder, &seeds, &request.canopy_api_key).await
                } else {
                    emit(&app, "  ⚠ No DataForSEO or Canopy credentials — skipping keyword search.");
                    Vec::new()
                }
            }
        };
        if !keyword_pool.is_empty() {
            let conn = database.0.lock().unwrap();
            let ks_json = serde_json::json!({
                "schema": "keyword_search_v1",
                "keywords": keyword_pool.iter().map(|k| serde_json::json!({
                    "keyword": k.keyword, "searches": k.searches, "competition": k.competition, "earnings": k.estimated_earnings,
                })).collect::<Vec<_>>(),
            }).to_string();
            save_selected_report(&conn, &request.folder, "keyword_search", &ks_json, &run_ts, &manuscript_fp);
            let _ = db::record_artifact_built(&conn, &request.folder, "keyword_search", &manuscript_fp);
        }
        if crate::is_cancelled() { return err("Cancelled."); }
    }

    // ── Step 8: KDP Keywords (KDP only) ────────────────────────────────────
    let mut kdp_keyword_entries: Vec<db::KdpKeywordEntry> = Vec::new();
    let mut kdp_keyword_strategy = String::new();
    if should_run_kdp_keywords(selected, platform) {
        emit(&app, "Step 8: Optimizing KDP keywords...");
        let res = call_keyword_optimizer_with_pool(&database, &request.provider, &request.api_key, &request.model, &genre_data, &genre_data.genre_signals, &keyword_pool).await;

        match res {
            Ok((entries, strategy)) => {
                let source_note = if keyword_pool.is_empty() {
                    "*(Generated from genre analysis — no keyword search data available.)*"
                } else {
                    "*(Enhanced with real Amazon search volume data.)*"
                };
                let conn = database.0.lock().unwrap();
                let _ = db::save_kdp_keywords(&conn, &request.folder, &entries, &strategy, source_note);
                emit(&app, &format!("  ✓ {} KDP keyword strings saved.", entries.len()));
                kdp_keyword_entries = entries;
                kdp_keyword_strategy = strategy;
            }
            Err(e) => {
                emit(&app, &format!("  ⚠ KDP keyword optimization failed: {} — continuing.", e));
            }
        }
        if crate::is_cancelled() { return err("Cancelled."); }
    }

    // ── Step 9: Discovery Keywords ─────────────────────────────────────────
    let mut discovery_entries: Vec<db::DiscoveryKeywordEntry> = Vec::new();
    if should_run_discovery_keywords(selected, platform, formats) {
        emit(&app, "Step 9: Generating discovery keywords...");
        let res = generate_discovery_keywords(&database, &request.provider, &request.api_key, &request.model, &genre_data).await;

        discovery_entries = match res {
            Ok(entries) => {
                let enriched = if !request.dataforseo_login.is_empty() && !request.dataforseo_password.is_empty() && !entries.is_empty() {
                    emit(&app, "  Enriching with Google search volume via DataForSEO...");
                    let phrases: Vec<String> = entries.iter().map(|e| e.phrase.clone()).collect();
                    let client = crate::dataforseo::DataForSeoClient::new(&request.dataforseo_login, &request.dataforseo_password);
                    match client {
                        Ok(c) => match c.google_search_volume(&phrases).await {
                            Ok(volumes) => {
                                entries.into_iter().map(|mut e| {
                                    if let Some(v) = volumes.iter().find(|v| v.keyword.to_lowercase() == e.phrase.to_lowercase()) {
                                        e.rationale = format!("{}/mo Google — {}", v.search_volume, e.rationale);
                                    }
                                    e
                                }).collect()
                            }
                            Err(err) => { emit(&app, &format!("  ⚠ DataForSEO volume lookup failed: {}", err)); entries }
                        }
                        Err(err) => { emit(&app, &format!("  ⚠ DataForSEO client error: {}", err)); entries }
                    }
                } else {
                    entries
                };

                let conn = database.0.lock().unwrap();
                let _ = db::save_discovery_keywords(&conn, &request.folder, &enriched);
                let dk_json = serde_json::json!({
                    "schema": "discovery_keywords_v1",
                    "keywords": enriched.iter().map(|e| serde_json::json!({ "phrase": e.phrase, "rationale": e.rationale })).collect::<Vec<_>>(),
                }).to_string();
                save_selected_report(&conn, &request.folder, "discovery_keywords", &dk_json, &run_ts, &manuscript_fp);
                let _ = db::record_artifact_built(&conn, &request.folder, "discovery_keywords", &manuscript_fp);
                emit(&app, &format!("  ✓ {} discovery keywords saved.", enriched.len()));
                enriched
            }
            Err(e) => {
                emit(&app, &format!("  ⚠ Discovery keywords failed: {} — continuing.", e));
                Vec::new()
            }
        };
        if crate::is_cancelled() { return err("Cancelled."); }
    }

    // ── Step 9b: Google Keyword Search (Wide only) ─────────────────────────
    let mut google_keywords_section = String::new();
    if should_run_google_keyword_search(selected, platform, formats) {
        emit(&app, "Step 9b: Google keyword search...");
        let discovery_phrases: Vec<String> = if !discovery_entries.is_empty() {
            discovery_entries.iter().map(|e| e.phrase.clone()).collect()
        } else {
            let conn = database.0.lock().unwrap();
            db::load_discovery_keywords(&conn, &request.folder)
                .into_iter()
                .map(|e| e.phrase)
                .collect()
        };
        let seeds = derive_wide_keyword_seeds(&genre_data.industry_ebook, &discovery_phrases);
        if seeds.is_empty() {
            emit(&app, "  ⚠ No seeds derived — skipping Google keyword search.");
        } else if request.dataforseo_login.is_empty() || request.dataforseo_password.is_empty() {
            emit(&app, "  ⚠ DataForSEO credentials not set — add login/password in Settings.");
        } else {
            emit(&app, &format!("  Seeds: {:?}", seeds));
            let results = run_google_keyword_searches_dataforseo(
                &app,
                &request.folder,
                &seeds,
                &request.dataforseo_login,
                &request.dataforseo_password,
            ).await;
            if !results.is_empty() {
                let conn = database.0.lock().unwrap();
                let ks_json = serde_json::json!({
                    "schema": "google_keyword_search_v1",
                    "keywords": results.iter().map(|k| serde_json::json!({
                        "keyword": k.keyword,
                        "searches": k.searches,
                        "competition": k.competition,
                        "cpc": k.estimated_earnings,
                    })).collect::<Vec<_>>(),
                }).to_string();
                google_keywords_section = ks_json.clone();
                save_selected_report(&conn, &request.folder, "google_keyword_search", &ks_json, &run_ts, &manuscript_fp);
                let _ = db::record_artifact_built(&conn, &request.folder, "google_keyword_search", &manuscript_fp);
                emit(&app, &format!("  ✓ {} Google keywords saved.", results.len()));
            }
        }
        if crate::is_cancelled() { return err("Cancelled."); }
    }

    // ── Step 9c: Content & Maturity Advisory (Wide only) ───────────────────
    let mut content_advisory_section = String::new();
    if should_run_content_advisory(selected, platform, formats) {
        emit(&app, "Step 9c: Content & maturity advisory...");
        let summaries = { let conn = database.0.lock().unwrap(); db::load_chapter_summaries(&conn, &request.folder) };
        let signals = aggregate_content_signals(&summaries);
        match generate_content_maturity_advisory(
            &database,
            &request.provider,
            &request.api_key,
            &request.model,
            &genre_data,
            &signals,
        ).await {
            Ok(value) => {
                let json = value.to_string();
                content_advisory_section = json.clone();
                let conn = database.0.lock().unwrap();
                save_selected_report(&conn, &request.folder, "content_maturity_advisory", &json, &run_ts, &manuscript_fp);
                emit(&app, "  ✓ Content & maturity advisory saved.");
            }
            Err(e) => emit(&app, &format!("  ⚠ Content advisory failed: {} — continuing.", e)),
        }
        if crate::is_cancelled() { return err("Cancelled."); }
    }

    // ── Step 9d: Wide Metadata Paste Sheet ───────────────────────────────────
    let mut wide_paste_section = String::new();
    if should_run_wide_paste(selected, platform) {
        emit(&app, "Step 9d: Assembling wide metadata paste sheet...");
        let (ebook_bisac, print_bisac, discovery_for_paste, content_note) = {
            let conn = database.0.lock().unwrap();
            let ebook = db::load_bisac_classifications(&conn, &request.folder, "ebook");
            let print = db::load_bisac_classifications(&conn, &request.folder, "print");
            let discovery = if !discovery_entries.is_empty() {
                discovery_entries.clone()
            } else {
                db::load_discovery_keywords(&conn, &request.folder)
            };
            let content_note = if !content_advisory_section.is_empty() {
                serde_json::from_str::<serde_json::Value>(&content_advisory_section).ok()
                    .and_then(|v| v["maturity_rating_suggestion"].as_str().map(String::from))
            } else {
                db::get_document(&conn, &request.folder, "content_maturity_advisory")
                    .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
                    .and_then(|v| v["maturity_rating_suggestion"].as_str().map(String::from))
            };
            (ebook, print, discovery, content_note)
        };
        if ebook_bisac.is_empty() && print_bisac.is_empty() && discovery_for_paste.is_empty() {
            emit(&app, "  ⚠ BISAC and discovery data not ready — run Wide Analysis or its dependencies.");
        } else {
            wide_paste_section = render_wide_paste_section(
                &genre_data,
                &ebook_bisac,
                &print_bisac,
                &discovery_for_paste,
                content_note.as_deref(),
            );
            if wants_report(selected, "wide_metadata_paste", platform)
                && !wants_wide_analysis_bundle(selected, platform)
            {
                let conn = database.0.lock().unwrap();
                save_selected_report(&conn, &request.folder, "wide_metadata_paste", &wide_paste_section, &run_ts, &manuscript_fp);
                emit(&app, "  ✓ Wide metadata paste sheet saved.");
            }
        }
        if crate::is_cancelled() { return err("Cancelled."); }
    }

    // ── Step 9e: Market Intel (KDP — competition, reviews, authors) ────────
    if platform == "kdp" && !request.canopy_api_key.is_empty() {
        if wants_report(selected, "competition_report", platform) {
            emit(&app, "Step 9e: Competition analysis...");
            let result = crate::canopy::analyze_competition_canopy(
                app.clone(),
                crate::canopy::CompetitionCanopyRequest {
                    folder: request.folder.clone(),
                    api_key: request.api_key.clone(),
                    model: request.model.clone(),
                    store: "Kindle".to_string(),
                    provider: request.provider.clone(),
                    canopy_api_key: request.canopy_api_key.clone(),
                },
            ).await;
            if result.success {
                let conn = database.0.lock().unwrap();
                if let Some(content) = db::get_document(&conn, &request.folder, "competition_report") {
                    save_selected_report(&conn, &request.folder, "competition_report", &content, &run_ts, &manuscript_fp);
                }
                emit(&app, "  ✓ Competition analysis saved.");
            } else {
                emit(&app, &format!("  ⚠ Competition analysis failed: {}", result.error));
            }
        }
        if wants_report(selected, "review_mining", platform) {
            emit(&app, "Step 9e: Review mining...");
            let result = crate::canopy::mine_competitor_reviews(
                app.clone(),
                crate::canopy::ReviewMiningRequest {
                    folder: request.folder.clone(),
                    canopy_api_key: request.canopy_api_key.clone(),
                    api_key: request.api_key.clone(),
                    model: request.model.clone(),
                    provider: request.provider.clone(),
                },
            ).await;
            if result.success {
                let conn = database.0.lock().unwrap();
                if let Some(content) = db::get_document(&conn, &request.folder, "review_mining") {
                    save_selected_report(&conn, &request.folder, "review_mining", &content, &run_ts, &manuscript_fp);
                }
                emit(&app, "  ✓ Review mining saved.");
            } else {
                emit(&app, &format!("  ⚠ Review mining failed: {}", result.error));
            }
        }
        if wants_report(selected, "author_analysis", platform) {
            emit(&app, "Step 9e: Author analysis...");
            let result = crate::canopy::analyze_comp_authors(
                app.clone(),
                crate::canopy::AuthorAnalysisRequest {
                    folder: request.folder.clone(),
                    canopy_api_key: request.canopy_api_key.clone(),
                    api_key: request.api_key.clone(),
                    model: request.model.clone(),
                    provider: request.provider.clone(),
                },
            ).await;
            if result.success {
                let conn = database.0.lock().unwrap();
                if let Some(content) = db::get_document(&conn, &request.folder, "author_analysis") {
                    save_selected_report(&conn, &request.folder, "author_analysis", &content, &run_ts, &manuscript_fp);
                }
                emit(&app, "  ✓ Author analysis saved.");
            } else {
                emit(&app, &format!("  ⚠ Author analysis failed: {}", result.error));
            }
        }
        if crate::is_cancelled() { return err("Cancelled."); }
    } else if platform == "kdp" && (
        wants_report(selected, "competition_report", platform)
        || wants_report(selected, "review_mining", platform)
        || wants_report(selected, "author_analysis", platform)
    ) {
        emit(&app, "  ⚠ Canopy API key required for market intel reports — add in Settings.");
    }

    let wants_kdp_bundle = wants_kdp_analysis_bundle(selected, platform);
    let wants_wide_bundle = wants_wide_analysis_bundle(selected, platform);
    if !wants_kdp_bundle && !wants_wide_bundle {
        emit(&app, "✓ Selected reports complete.");
        return GenreResult { success: true, report: String::new(), error: String::new(), run_ts: run_ts.clone() };
    }

    let description_snippet = {
        let conn = database.0.lock().unwrap();
        db::get_document(&conn, &request.folder, "blurb_builder")
            .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
            .and_then(|v| v["variants"].as_array().and_then(|a| a.first()).and_then(|x| x["blurb"].as_str().map(String::from)))
    };

    let discovery_keywords_section = serde_json::json!({
        "keywords": discovery_entries.iter().map(|e| serde_json::json!({
            "phrase": e.phrase,
            "rationale": e.rationale,
        })).collect::<Vec<_>>(),
    }).to_string();

    let positioning_section = serde_json::json!({
        "reader_demographic": genre_data.reader_demographic,
        "bookstore_shelving": genre_data.bookstore_shelving,
        "comps_ebook": genre_data.comps_ebook,
        "comps_print": genre_data.comps_print,
    }).to_string();

    if wants_wide_bundle {
        emit(&app, "Step 10: Assembling Wide Analysis report...");
        if wide_paste_section.is_empty() && should_run_wide_paste(selected, platform) {
            let conn = database.0.lock().unwrap();
            let ebook = db::load_bisac_classifications(&conn, &request.folder, "ebook");
            let print = db::load_bisac_classifications(&conn, &request.folder, "print");
            let discovery = if !discovery_entries.is_empty() {
                discovery_entries.clone()
            } else {
                db::load_discovery_keywords(&conn, &request.folder)
            };
            let content_note = if !content_advisory_section.is_empty() {
                serde_json::from_str::<serde_json::Value>(&content_advisory_section).ok()
                    .and_then(|v| v["maturity_rating_suggestion"].as_str().map(String::from))
            } else {
                None
            };
            wide_paste_section = render_wide_paste_section(
                &genre_data, &ebook, &print, &discovery, content_note.as_deref(),
            );
        }
        let report = render_wide_combined_report(
            &genre_ranking_section,
            &bisac_section,
            &discovery_keywords_section,
            &google_keywords_section,
            &content_advisory_section,
            &wide_paste_section,
            &positioning_section,
        );
        {
            let conn = database.0.lock().unwrap();
            let _ = db::save_document_current(&conn, &request.folder, "wide_analysis", &report, &run_ts, &manuscript_fp);
            emit(&app, "✓ Wide Analysis report saved.");
        }
        if !wants_kdp_bundle {
            return GenreResult { success: true, report, error: String::new(), run_ts: run_ts.clone() };
        }
    }

    if wants_kdp_bundle {
    // ── Step 10: Assemble KDP Combined Report ───────────────────────────────
    emit(&app, "Step 10: Assembling KDP Analysis report...");

    let bisac_print_json = if bisac_section.is_empty() {
        None
    } else {
        serde_json::from_str::<serde_json::Value>(&bisac_section).ok()
            .and_then(|v| if v["print"].is_string() { v.get("ebook").map(|x| x.to_string()) } else { v.get("print").map(|x| x.to_string()) })
    };
    let kdp_paste = render_kdp_paste_section(
        &kindle_top_categories,
        &print_top_categories,
        &kdp_keyword_entries,
        bisac_print_json.as_deref(),
        description_snippet.as_deref(),
    );

    let source_note = if keyword_pool.is_empty() {
        "*(Generated from genre analysis — no keyword search data available.)*"
    } else {
        "*(Enhanced with real Amazon search volume data.)*"
    };
    let kdp_keywords_section = render_kdp_keywords(&kdp_keyword_entries, &kdp_keyword_strategy, source_note);

    let report = render_combined_report(
        &kdp_paste,
        &genre_ranking_section,
        &kdp_categories_section,
        &bisac_section,
        &kdp_keywords_section,
        &discovery_keywords_section,
        &positioning_section,
        description_snippet.as_deref(),
    );

    {
        let conn = database.0.lock().unwrap();
        let _ = db::save_document_current(&conn, &request.folder, "analysis", &report, &run_ts, &manuscript_fp);
        emit(&app, "✓ KDP Analysis report saved.");
    }

    return GenreResult { success: true, report, error: String::new(), run_ts: run_ts.clone() };
    }

    GenreResult { success: true, report: String::new(), error: String::new(), run_ts: run_ts.clone() }
}

// ── KDP Paste Section Renderer ─────────────────────────────────────────────────

/// Renders the "KDP Metadata — Ready to Paste" section that mirrors the KDP
/// website's actual input layout.
pub(crate) fn render_kdp_paste_section(
    kindle_categories: &[String],
    print_categories: &[String],
    keywords: &[db::KdpKeywordEntry],
    bisac_print_json: Option<&str>,
    description_snippet: Option<&str>,
) -> String {
    let bisac_print: serde_json::Value = bisac_print_json
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(serde_json::json!([]));
    let json = serde_json::json!({
        "schema": "kdp_paste_v1",
        "kindle_categories": kindle_categories.iter().take(3).collect::<Vec<_>>(),
        "print_categories": print_categories.iter().take(3).collect::<Vec<_>>(),
        "keywords": keywords.iter().map(|k| &k.string).collect::<Vec<_>>(),
        "bisac_print": bisac_print,
        "description_snippet": description_snippet.unwrap_or(""),
    });
    json.to_string()
}

/// Renders copy-ready wide-distribution metadata (BISAC, discovery keywords, content note).
pub(crate) fn render_wide_paste_section(
    genre_data: &db::GenreDataRow,
    ebook_bisac: &[(String, String, u8, String)],
    print_bisac: &[(String, String, u8, String)],
    discovery: &[db::DiscoveryKeywordEntry],
    content_note: Option<&str>,
) -> String {
    let json = serde_json::json!({
        "schema": "wide_paste_v1",
        "genre_labels": {
            "ebook": genre_data.industry_ebook,
            "print": genre_data.industry_print,
        },
        "bisac_ebook": ebook_bisac.iter().map(|(code, heading, conf, _)| serde_json::json!({
            "code": code,
            "heading": heading,
            "confidence": conf,
        })).collect::<Vec<_>>(),
        "bisac_print": print_bisac.iter().map(|(code, heading, conf, _)| serde_json::json!({
            "code": code,
            "heading": heading,
            "confidence": conf,
        })).collect::<Vec<_>>(),
        "discovery_keywords": discovery.iter().map(|e| &e.phrase).collect::<Vec<_>>(),
        "content_note": content_note.unwrap_or(""),
    });
    json.to_string()
}

// ── Craft pipeline ────────────────────────────────────────────────────────────

/// Request for the craft analysis pipeline.
/// The frontend sends which reports to run; this command handles ordering and execution.
#[derive(serde::Deserialize)]
pub struct CraftPipelineRequest {
    pub folder:           String,
    pub selected:         Vec<String>,
    pub provider:         String,
    pub api_key:          String,
    pub model:            String,           // default fallback
    #[serde(default)]
    pub model_summaries:  String,           // override for chapter summaries
    #[serde(default)]
    pub model_continuity: String,           // override for continuity check
    #[serde(default)]
    pub model_sdt:        String,           // override for show don't tell
    #[serde(default)]
    pub model_ai_isms:    String,           // override for AI-isms check
    #[serde(default)]
    pub model_prose:      String,           // override for prose / creative copy (AI Beta Reader, Blurb Builder)
    /// "manuscript" or "series"
    #[serde(default)]
    pub continuity_scope: String,
    /// Only used when continuity_scope == "series"
    #[serde(default)]
    pub series_id:        i64,
    #[serde(default)]
    pub bible_path:       String,
}

/// Runs the selected craft-platform reports in the correct order.
/// Chapter summaries → Zeigarnik → Continuity. Each is optional based on `selected`.
#[tauri::command]
pub async fn run_craft_pipeline(app: AppHandle, request: CraftPipelineRequest) -> GenreResult {
    let cancel = crate::cancel_notify();
    tokio::select! {
        result = run_craft_pipeline_inner(app, request) => result,
        _ = cancel.notified() => err("Cancelled."),
    }
}

async fn run_craft_pipeline_inner(app: AppHandle, request: CraftPipelineRequest) -> GenreResult {
    let folder = PathBuf::from(&request.folder);
    if !folder.exists() { return err("Folder does not exist."); }

    // Resolve per-function models (fall back to default)
    let model_continuity = if request.model_continuity.is_empty() { &request.model } else { &request.model_continuity };
    let model_summaries = if request.model_summaries.is_empty() { &request.model } else { &request.model_summaries };
    let model_sdt = if request.model_sdt.is_empty() { &request.model } else { &request.model_sdt };
    let model_ai_isms = if request.model_ai_isms.is_empty() { &request.model } else { &request.model_ai_isms };

    crate::reset_cancel();
    let database = app.state::<db::Db>();
    let run_ts = chrono::Utc::now().to_rfc3339();
    let needs_ai = request.selected.iter().any(|s| {
        s == "continuity_check"
            || s == "show_dont_tell"
            || s == "ai_isms"
            || super::craft_audits::is_craft_audit(s)
            || matches!(s.as_str(), "ai_beta_reader" | "cliffhanger_score" | "hook_strength" | "pacing_curve" | "blurb_builder")
    });

    if needs_ai {
        if let Err(msg) = crate::ai::ai_ready(&request.provider, &request.api_key, &request.model) {
            return err(&msg);
        }
    }

    // ── Chapter summaries (infrastructure — not saved as a report) ───────
    if request.selected.contains(&"chapter_summaries".to_string()) {
        emit(&app, "Summarizing chapters (AI)...");
        let chapters = collect_chapters(&folder);
        if chapters.is_empty() {
            return err("No .md chapter files found.");
        }

        let phase1_config = phase1_config_from(
            &request.provider,
            &request.api_key,
            &request.model,
            &request.model_summaries,
            false,
        );
        let (done, skipped) =
            phase1_summaries(&app, &database, &chapters, &request.folder, &phase1_config).await;
        emit(
            &app,
            &format!("✓ Chapter summaries complete ({} new, {} skipped).", done, skipped),
        );

        let manuscript_fp = compute_manuscript_fingerprint(&folder);
        let conn = database.0.lock().unwrap();
        let _ = db::record_artifact_built(&conn, &request.folder, "summaries", &manuscript_fp);

        if crate::is_cancelled() {
            return err("Cancelled.");
        }
    }

    // ── Zeigarnik Effect ──────────────────────────────────────────────────
    if request.selected.contains(&"zeigarnik_analysis".to_string()) {
        emit(&app, "Running Zeigarnik effect analysis (algorithmic — no AI)...");
        let zr = super::zeigarnik::analyze_zeigarnik_for_story(
            app.clone(),
            super::zeigarnik::ZeigarnikRequest { folder: request.folder.clone() },
        ).await;
        if zr.success {
            emit(&app, "✓ Zeigarnik analysis complete.");
        } else {
            emit(&app, &format!("✗ Zeigarnik: {}", zr.error));
            return zr;
        }
        if crate::is_cancelled() { return err("Cancelled."); }
    }

    // ── Continuity Check ──────────────────────────────────────────────────
    if request.selected.contains(&"continuity_check".to_string()) {
        if request.continuity_scope == "series" && request.series_id > 0 {
            emit(&app, &format!("Running continuity check across the series... [{}: {}]", request.provider, model_continuity));
            let cr = super::continuity::check_continuity_for_series(
                app.clone(),
                super::continuity::SeriesContinuityRequest {
                    series_id: request.series_id,
                    provider: request.provider.clone(),
                    api_key: request.api_key.clone(),
                    model: model_continuity.clone(),
                    extraction_model: model_summaries.clone(),
                    bible_path: request.bible_path.clone(),
                },
            ).await;
            if cr.success {
                emit(&app, "✓ Series continuity check complete.");
            } else {
                emit(&app, &format!("✗ Continuity: {}", cr.error));
                return cr;
            }
        } else {
            emit(&app, &format!("Running continuity check for this manuscript... [{}: {}]", request.provider, model_continuity));
            let cr = super::continuity::check_continuity_for_story(
                app.clone(),
                super::continuity::ContinuityRequest {
                    folder: request.folder.clone(),
                    provider: request.provider.clone(),
                    api_key: request.api_key.clone(),
                    model: model_continuity.clone(),
                    extraction_model: model_summaries.clone(),
                    bible_path: request.bible_path.clone(),
                },
            ).await;
            if cr.success {
                emit(&app, "✓ Continuity check complete.");
            } else {
                emit(&app, &format!("✗ Continuity: {}", cr.error));
                return cr;
            }
        }
        if crate::is_cancelled() { return err("Cancelled."); }
    }

    // ── Show Don't Tell + AI-isms (combined when both selected) ───────────
    let wants_sdt = request.selected.contains(&"show_dont_tell".to_string());
    let wants_ai = request.selected.contains(&"ai_isms".to_string());

    if wants_sdt && wants_ai {
        emit(&app, "Running combined show-don't-tell + AI-isms check (single batched pass)...");
        let craft_model = if !request.model_sdt.is_empty() {
            model_sdt.clone()
        } else {
            model_ai_isms.clone()
        };
        let combined = super::craft_prose_checks::check_craft_prose_combined(
            app.clone(),
            super::craft_prose_checks::CraftProseChecksRequest {
                folder: request.folder.clone(),
                provider: request.provider.clone(),
                api_key: request.api_key.clone(),
                model: craft_model,
                bible_path: request.bible_path.clone(),
            },
        )
        .await;
        if !combined.success {
            emit(&app, &format!("✗ Craft prose checks: {}", combined.error));
            return combined;
        }
        if crate::is_cancelled() {
            return err("Cancelled.");
        }
    } else if wants_sdt {
        let sdt = super::show_dont_tell::check_show_dont_tell(
            app.clone(),
            super::show_dont_tell::ShowDontTellRequest {
                folder: request.folder.clone(),
                provider: request.provider.clone(),
                api_key: request.api_key.clone(),
                model: model_sdt.clone(),
                bible_path: request.bible_path.clone(),
            },
        )
        .await;
        if !sdt.success {
            emit(&app, &format!("✗ Show Don't Tell: {}", sdt.error));
            return sdt;
        }
        if crate::is_cancelled() {
            return err("Cancelled.");
        }
    } else if wants_ai {
        let ai = super::ai_isms::check_ai_isms(
            app.clone(),
            super::ai_isms::AiIsmsRequest {
                folder: request.folder.clone(),
                provider: request.provider.clone(),
                api_key: request.api_key.clone(),
                model: model_ai_isms.clone(),
                bible_path: request.bible_path.clone(),
            },
        )
        .await;
        if !ai.success {
            emit(&app, &format!("✗ AI-isms: {}", ai.error));
            return ai;
        }
        if crate::is_cancelled() {
            return err("Cancelled.");
        }
    }

    // ── Generic craft audits (StoryAuditor catalog) ───────────────────────
    let model_craft = model_continuity;
    for audit_id in super::craft_audits::MANUSCRIPT_AUDITS {
        if !request.selected.iter().any(|s| s == *audit_id) { continue; }
        let r = super::craft_audits::run_manuscript_craft_audit(
            &app, &database, &request.folder, audit_id,
            &request.provider, &request.api_key, model_craft, &request.bible_path,
        ).await;
        if !r.success { return r; }
        if crate::is_cancelled() { return err("Cancelled."); }
    }
    for audit_id in super::craft_audits::SERIES_AUDITS {
        if !request.selected.iter().any(|s| s == *audit_id) { continue; }
        let r = super::craft_audits::run_series_craft_audit(
            &app, &database, request.series_id, audit_id,
            &request.provider, &request.api_key, model_craft, &request.bible_path,
        ).await;
        if !r.success { return r; }
        if crate::is_cancelled() { return err("Cancelled."); }
    }

    // ── Publish platform reports ──────────────────────────────────────────
    let model_publish = if request.model_summaries.is_empty() { &request.model } else { &request.model_summaries };
    let model_prose = if request.model_prose.is_empty() { &request.model } else { &request.model_prose };

    if request.selected.iter().any(|s| s == "ai_beta_reader") {
        let r = super::publish_audits::run_ai_beta_reader(
            &app, &database, &request.folder,
            &request.provider, &request.api_key, model_prose, &request.bible_path,
        ).await;
        if !r.success { return r; }
        if crate::is_cancelled() { return err("Cancelled."); }
    }
    if request.selected.iter().any(|s| s == "cliffhanger_score") {
        let r = super::publish_audits::run_cliffhanger_score(
            &app, &database, &request.folder,
            &request.provider, &request.api_key, model_publish,
        ).await;
        if !r.success { return r; }
        if crate::is_cancelled() { return err("Cancelled."); }
    }
    if request.selected.iter().any(|s| s == "hook_strength") {
        let r = super::publish_audits::run_hook_strength(
            &app, &database, &request.folder,
            &request.provider, &request.api_key, model_publish, &request.bible_path,
        ).await;
        if !r.success { return r; }
        if crate::is_cancelled() { return err("Cancelled."); }
    }
    if request.selected.iter().any(|s| s == "pacing_curve") {
        let r = super::publish_audits::run_pacing_curve(
            &app, &database, &request.folder,
            &request.provider, &request.api_key, model_publish,
        ).await;
        if !r.success { return r; }
        if crate::is_cancelled() { return err("Cancelled."); }
    }
    if request.selected.iter().any(|s| s == "line_polish") {
        let r = super::publish_audits::run_line_polish(&app, &database, &request.folder);
        if !r.success { return r; }
    }
    if request.selected.iter().any(|s| s == "vellum_prep") {
        let r = super::publish_audits::run_vellum_prep(&app, &database, &request.folder);
        if !r.success { return r; }
    }
    if request.selected.iter().any(|s| s == "blurb_builder") {
        let r = super::publish_audits::run_blurb_builder(
            &app, &database, &request.folder,
            &request.provider, &request.api_key, model_prose,
        ).await;
        if !r.success { return r; }
        if crate::is_cancelled() { return err("Cancelled."); }
    }
    if request.selected.iter().any(|s| s == "print_production") {
        let r = super::publish_audits::run_print_production(&app, &database, &request.folder);
        if !r.success { return r; }
    }

    emit(&app, "✓ Done.");
    GenreResult { success: true, report: String::new(), error: String::new(), run_ts }
}

#![deny(dead_code)]

mod ai;
mod analysis;
mod cancel;
mod canopy;
mod commands;
mod competition_analyzer;
mod dataforseo;
mod db;
mod folder_structure;
mod genre_taxonomy;
mod models;
mod llm;
mod batch_prompt;
mod prompts;
mod campaigns;
mod series;
mod settings;
mod stories;
mod winningcat;

use tauri::Manager;

pub use cancel::{is_cancelled, reset as reset_cancel, notify as cancel_notify};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let database = db::init(&handle).expect("failed to initialize database");
            app.manage(database);
            let _ = folder_structure::load(&handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::analyze_csv,
            commands::list_models,
            commands::read_chapter,
            commands::save_chapter,
            commands::write_manuscript_fix,
            commands::list_manuscript_files,
            commands::estimate_report_costs,
            commands::estimate_summary_refresh_cost,
            commands::chat_with_context,
            analysis::chapters::generate_summaries,
            analysis::genres::analyze_genre,
            analysis::genres::rank_genres_for_story,
            analysis::categories::find_categories_for_story,
            analysis::categories::match_categories_for_story,
            analysis::categories::verify_mapped_categories,
            analysis::bisac::classify_bisac_for_story,
            analysis::keywords::generate_search_terms,
            analysis::keywords::optimize_keywords,
            analysis::pipeline::pick_manuscript_folder,
            analysis::pipeline::check_analysis_state,
            analysis::pipeline::run_everything,
            analysis::pipeline::run_full_analysis,
            analysis::pipeline::find_genres_and_categories_for_story,
            analysis::pipeline::analyze_story,
            analysis::pipeline::run_craft_pipeline,
            analysis::zeigarnik::analyze_zeigarnik_for_story,
            analysis::continuity::check_continuity_for_story,
            analysis::continuity::check_continuity_for_series,
            analysis::continuity::suggest_continuity_fix,
            analysis::show_dont_tell::check_show_dont_tell,
            analysis::show_dont_tell::suggest_sdt_fix,
            analysis::ai_isms::check_ai_isms,
            analysis::ai_isms::suggest_ai_isms_fix,
            analysis::craft_prose_checks::check_craft_prose_combined,
            genre_taxonomy::get_genre_taxonomy,
            db::list_genres_cmd,
            db::list_report_types_cmd,
            db::add_kdp_path_cmd,
            db::list_reports_cmd,
            db::save_activity_log_cmd,
            db::get_report_cmd,
            db::delete_report_cmd,
            db::get_sidebar_reports,
            db::get_archived_reports,
            db::get_story_artifact_state,
            db::refresh_chapter_fingerprints,
            db::clear_chapter_fingerprints,
            db::inspect_database_overview,
            db::inspect_database_table,
            db::delete_database_row_cmd,
            db::update_database_row_cmd,
            settings::load_ui_settings,
            settings::save_ui_settings,
            settings::load_app_state,
            settings::save_app_state,
            db::list_series_cmd,
            db::create_series_cmd,
            db::delete_series_cmd,
            db::list_series_books_cmd,
            db::add_story_to_series_cmd,
            db::remove_story_from_series_cmd,
            winningcat::import_winningcat_csv,
            winningcat::remove_stale_kdp_categories,
            cancel::cancel_operation,
            canopy::test_canopy_connection,
            canopy::analyze_categories_canopy,
            canopy::analyze_competition_canopy,
            canopy::search_keywords_canopy,
            canopy::mine_competitor_reviews,
            canopy::analyze_comp_authors,
            canopy::deep_category_analysis,
            canopy::run_market_intel,
            dataforseo::test_dataforseo_connection,
            dataforseo::search_amazon_keywords,
            dataforseo::search_google_keywords,
            stories::list_stories,
            stories::add_story,
            stories::init_story,
            stories::update_story,
            stories::delete_story,
            stories::create_story_document,
            stories::delete_story_document,
            folder_structure::get_folder_structure,
            folder_structure::save_folder_structure,
            series::list_series,
            series::create_series,
            series::update_series,
            series::delete_series,
            campaigns::list_campaigns,
            campaigns::create_campaign,
            campaigns::update_campaign,
            campaigns::delete_campaign,
            campaigns::get_campaign_detail,
            campaigns::list_creatives,
            campaigns::create_creative,
            campaigns::update_creative,
            campaigns::delete_creative,
            campaigns::list_performance_snapshots,
            campaigns::add_performance_snapshot,
            campaigns::delete_performance_snapshot,
            campaigns::list_spend_entries,
            campaigns::add_spend_entry,
            campaigns::delete_spend_entry,
            campaigns::list_landing_pages,
            campaigns::create_landing_page,
            campaigns::update_landing_page,
            campaigns::delete_landing_page,
            campaigns::list_audience_notes,
            campaigns::add_audience_note,
            campaigns::update_audience_note,
            campaigns::delete_audience_note,
            campaigns::list_platform_accounts,
            campaigns::create_platform_account,
            campaigns::update_platform_account,
            campaigns::delete_platform_account,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, _event| {});
}

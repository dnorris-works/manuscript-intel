// db.rs — SQLite-backed storage for the genre/category reference system.
//
// Source of truth for: story registry, genres, KDP category paths,
// genre<->category links, and per-story analysis/report state.
// Human-readable manuscript/report .md files still live in story folders for
// direct user access; the DB stores metadata and analysis records.
//
// The genre-list.json / genre-kdp-map.json files in src-tauri/data/ are used
// ONLY as one-time seed data on first launch (when the genres table is
// empty). After that, the database is authoritative — new categories
// discovered via Category Finder get written straight into it, so the
// genre-to-KDP-path map grows on its own with real, verified data instead of
// staying frozen at the hand-typed seed set.

use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

const SEED_GENRE_LIST_JSON:    &str = include_str!("../data/genre-list.json");
const SEED_GENRE_KDP_MAP_JSON: &str = include_str!("../data/genre-kdp-map.json");
const SEED_BISAC_JSON:         &str = include_str!("../data/bisac-fiction.json");
const SEED_ZEIGARNIK_CONFIG_JSON: &str = include_str!("../data/zeigarnik-config.json");
const SEED_PROMPT_TEMPLATES_JSON: &str = include_str!("../data/prompt-templates.json");
const SEED_PROVIDER_MODELS_JSON: &str = include_str!("../data/provider-models.json");
const SEED_LOOKUP_CONFIG_JSON: &str = include_str!("../data/lookup-config.json");

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS stories (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    folder     TEXT NOT NULL UNIQUE,
    created    TEXT NOT NULL,
    bible_path TEXT NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_stories_name ON stories(name);

CREATE TABLE IF NOT EXISTS genres (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL UNIQUE,
    description TEXT
);

CREATE TABLE IF NOT EXISTS kdp_categories (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    path           TEXT NOT NULL,
    store          TEXT NOT NULL DEFAULT 'Kindle',
    amazon_node_id TEXT,
    source         TEXT NOT NULL DEFAULT 'manual',   -- 'manual' | 'winningcat' | 'category_finder'
    verified_at    TEXT,                              -- last time confirmed live
    created_at     TEXT NOT NULL,
    UNIQUE(path, store)
);

CREATE TABLE IF NOT EXISTS genre_kdp_links (
    genre_id    INTEGER NOT NULL REFERENCES genres(id) ON DELETE CASCADE,
    category_id INTEGER NOT NULL REFERENCES kdp_categories(id) ON DELETE CASCADE,
    PRIMARY KEY (genre_id, category_id)
);

CREATE TABLE IF NOT EXISTS genre_rankings (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    story_folder TEXT NOT NULL,
    genre_id     INTEGER NOT NULL REFERENCES genres(id),
    confidence   INTEGER NOT NULL,
    reason       TEXT,
    generated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS category_results (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    story_folder  TEXT NOT NULL,
    category_id   INTEGER REFERENCES kdp_categories(id),
    raw_path      TEXT NOT NULL,
    store         TEXT NOT NULL,
    confidence    INTEGER NOT NULL,
    sales_to_one  TEXT,
    sales_to_ten  TEXT,
    publisher_pct TEXT,
    ku_pct        TEXT,
    status        TEXT NOT NULL,   -- 'matched' | 'considered' | 'failed'
    note          TEXT,
    generated_at  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_rankings_folder  ON genre_rankings(story_folder);
CREATE INDEX IF NOT EXISTS idx_results_folder   ON category_results(story_folder);
CREATE INDEX IF NOT EXISTS idx_categories_path  ON kdp_categories(path);
CREATE INDEX IF NOT EXISTS idx_categories_store ON kdp_categories(store);

CREATE TABLE IF NOT EXISTS chapter_summaries (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    story_folder TEXT NOT NULL,
    file         TEXT NOT NULL,
    title        TEXT,
    signals      TEXT,
    source_hash  TEXT NOT NULL DEFAULT '',
    word_count   INTEGER,
    updated_at   TEXT NOT NULL,
    UNIQUE(story_folder, file)
);

CREATE TABLE IF NOT EXISTS genre_data (
    story_folder         TEXT PRIMARY KEY,
    generated_at         TEXT NOT NULL,
    industry_ebook       TEXT,
    industry_print       TEXT,
    genre_signals        TEXT,
    reader_demographic   TEXT,
    bookstore_shelving   TEXT,
    kdp_ebook_json       TEXT NOT NULL DEFAULT '[]',
    kdp_print_json       TEXT NOT NULL DEFAULT '[]',
    comps_ebook_json     TEXT NOT NULL DEFAULT '[]',
    comps_print_json     TEXT NOT NULL DEFAULT '[]',
    marketing_notes_json TEXT NOT NULL DEFAULT '[]'
);

CREATE TABLE IF NOT EXISTS kdp_keywords (
    story_folder TEXT PRIMARY KEY,
    generated_at TEXT NOT NULL,
    keywords_json TEXT NOT NULL,   -- [{"string":..,"chars":..,"rationale":..}]
    strategy     TEXT,
    source_note  TEXT
);

CREATE TABLE IF NOT EXISTS mi_search_terms (
    story_folder  TEXT PRIMARY KEY,
    generated_at  TEXT NOT NULL,
    keywords_json TEXT NOT NULL   -- ["kw1","kw2",...]
);

CREATE TABLE IF NOT EXISTS discovery_keywords (
    story_folder  TEXT PRIMARY KEY,
    generated_at  TEXT NOT NULL,
    keywords_json TEXT NOT NULL   -- [{"phrase":..,"rationale":..}]
);

CREATE TABLE IF NOT EXISTS keyword_search_results (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    story_folder  TEXT NOT NULL,
    seed          TEXT NOT NULL,
    keyword       TEXT NOT NULL,
    searches      TEXT,
    competition   TEXT,
    earnings      TEXT,
    generated_at  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_keyword_results_folder ON keyword_search_results(story_folder);

-- Rendered markdown cache for the Reports panel. Every report type is
-- re-rendered fresh from its structured source table whenever regenerated —
-- this table is what the UI reads, never hand-edited, never stale-checked
-- against a file that quietly stopped being written (see: the genre-ranking
-- .json/.md drift this replaces).
CREATE TABLE IF NOT EXISTS story_documents (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    story_folder TEXT NOT NULL,
    doc_type     TEXT NOT NULL,
    content      TEXT NOT NULL,
    generated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_story_docs_folder ON story_documents(story_folder, doc_type);

CREATE INDEX IF NOT EXISTS idx_summaries_folder ON chapter_summaries(story_folder);

-- Structured chapter fingerprints (source of truth for Phase 1 scans).
CREATE TABLE IF NOT EXISTS chapter_fingerprints (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    story_folder    TEXT NOT NULL,
    file            TEXT NOT NULL,
    title           TEXT,
    source_hash     TEXT NOT NULL DEFAULT '',
    word_count      INTEGER NOT NULL DEFAULT 0,
    sentence_count  INTEGER NOT NULL DEFAULT 0,
    paragraph_count INTEGER NOT NULL DEFAULT 0,
    dialogue_pct    INTEGER NOT NULL DEFAULT 0,
    pov             TEXT NOT NULL DEFAULT '',
    tense           TEXT NOT NULL DEFAULT '',
    pacing          TEXT NOT NULL DEFAULT '',
    lexicon_json    TEXT NOT NULL DEFAULT '{}',
    updated_at      TEXT NOT NULL,
    UNIQUE(story_folder, file)
);

CREATE INDEX IF NOT EXISTS idx_fingerprints_folder ON chapter_fingerprints(story_folder);

CREATE TABLE IF NOT EXISTS bisac_codes (
    code    TEXT PRIMARY KEY,
    heading TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS bisac_classifications (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    story_folder TEXT NOT NULL,
    code         TEXT NOT NULL,
    heading      TEXT NOT NULL,
    confidence   INTEGER NOT NULL,
    reason       TEXT,
    generated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_bisac_folder ON bisac_classifications(story_folder);

-- Versioned saved reports. The user explicitly saves a report version via the
-- UI. Each save auto-increments the version number per story+doc_type pair.
-- Reports panel shows saved versions newest-first.
CREATE TABLE IF NOT EXISTS saved_reports (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    story_folder TEXT NOT NULL,
    doc_type     TEXT NOT NULL,
    version      INTEGER NOT NULL,
    label        TEXT NOT NULL,
    content      TEXT NOT NULL,
    saved_at     TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_saved_reports_folder ON saved_reports(story_folder, doc_type);

CREATE TABLE IF NOT EXISTS app_settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS report_types (
    id                TEXT PRIMARY KEY,
    label             TEXT NOT NULL,
    description       TEXT NOT NULL,
    platforms         TEXT NOT NULL DEFAULT 'kdp,wide',  -- comma-separated: 'kdp', 'wide', 'craft'
    depends_on        TEXT NOT NULL DEFAULT '',          -- comma-separated report_type ids
    cost_truncation   INTEGER NOT NULL DEFAULT 4000,     -- words of chapter text sent (0 = n/a)
    cost_output_max   INTEGER NOT NULL DEFAULT 1000,     -- max output tokens per call
    cost_per_chapter  INTEGER NOT NULL DEFAULT 0,        -- 1 = one LLM call per chapter
    cost_fixed_calls  INTEGER NOT NULL DEFAULT 1,        -- additional / fixed LLM calls
    model_slot        TEXT NOT NULL DEFAULT 'default',   -- Settings model assignment key
    min_tier          TEXT NOT NULL DEFAULT 'basic'      -- basic | capable | strong
);

-- Static provider model catalogs for seeded pricing fallbacks. TokenMix is fetched live.
CREATE TABLE IF NOT EXISTS provider_models (
    id           TEXT PRIMARY KEY,
    provider     TEXT NOT NULL,
    owned_by     TEXT NOT NULL DEFAULT '',
    input_price  REAL,
    output_price REAL,
    sort_order   INTEGER NOT NULL DEFAULT 0
);

-- Small JSON lookup lists (honorifics, etc.) editable without a rebuild.
CREATE TABLE IF NOT EXISTS lookup_config (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL   -- JSON-encoded
);

-- Zeigarnik effect detector: pure textual-proxy analysis, no AI. Phrase lists
-- and thresholds live here (seeded once from zeigarnik-config.json) instead of
-- being hardcoded in Rust, so they can be tuned per-project without a rebuild.
CREATE TABLE IF NOT EXISTS zeigarnik_config (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL   -- JSON-encoded: array of strings, or a single number/object
);

-- One row per manuscript chapter per analysis run (replaced wholesale each run).
CREATE TABLE IF NOT EXISTS zeigarnik_chapters (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    story_folder   TEXT NOT NULL,
    chapter_index  INTEGER NOT NULL,
    file           TEXT NOT NULL,
    title          TEXT NOT NULL,
    word_count     INTEGER NOT NULL,
    sentence_count INTEGER NOT NULL,
    question_count INTEGER NOT NULL,
    ending_type    TEXT NOT NULL,   -- 'cliffhanger' | 'neutral' | 'resolved'
    tension_score  INTEGER NOT NULL, -- 0-100, heuristic
    ending_snippet TEXT NOT NULL,
    generated_at   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_zeigarnik_chapters_folder ON zeigarnik_chapters(story_folder);

-- Candidate open loops: a capitalized term/phrase that reappears after a
-- long gap of chapters. A textual proxy for "unresolved thread the reader
-- may still be holding open" — not a direct measurement of recall.
CREATE TABLE IF NOT EXISTS zeigarnik_threads (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    story_folder       TEXT NOT NULL,
    term               TEXT NOT NULL,
    mention_count      INTEGER NOT NULL,
    first_chapter_index INTEGER NOT NULL,
    first_file         TEXT NOT NULL,
    first_snippet      TEXT NOT NULL,
    gap_start_index    INTEGER NOT NULL,  -- chapter after which the term went quiet
    gap_end_index      INTEGER NOT NULL,  -- chapter where it resurfaces
    max_gap_chapters   INTEGER NOT NULL,
    max_gap_words      INTEGER NOT NULL,
    generated_at       TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_zeigarnik_threads_folder ON zeigarnik_threads(story_folder);

-- Continuity Checker: groups stories into a series (reading order) so facts
-- can be compared across books, not just within one manuscript.
CREATE TABLE IF NOT EXISTS series (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS series_books (
    series_id    INTEGER NOT NULL REFERENCES series(id) ON DELETE CASCADE,
    story_folder TEXT NOT NULL,
    story_name   TEXT NOT NULL DEFAULT '',
    book_order   INTEGER NOT NULL,
    PRIMARY KEY (series_id, story_folder)
);

CREATE INDEX IF NOT EXISTS idx_series_books_series ON series_books(series_id);

-- AI-extracted continuity-relevant facts, one row per (entity, attribute,
-- chapter) triple. Extraction runs once per chapter; comparison (finding
-- contradictions) is a separate pass over the accumulated facts — see
-- continuity_findings below.
CREATE TABLE IF NOT EXISTS continuity_facts (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    story_folder  TEXT NOT NULL,
    chapter_index INTEGER NOT NULL,
    file          TEXT NOT NULL,
    chapter_title TEXT NOT NULL,
    entity        TEXT NOT NULL,
    entity_type   TEXT NOT NULL,   -- 'character' | 'place' | 'object' | 'timeline' | 'other'
    attribute     TEXT NOT NULL,
    value         TEXT NOT NULL,
    snippet       TEXT NOT NULL,
    generated_at  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_continuity_facts_folder ON continuity_facts(story_folder);

-- AI-judged contradictions. `scope` is 'manuscript' (scope_key = story_folder)
-- or 'series' (scope_key = 'series:<id>') so the same table serves both.
CREATE TABLE IF NOT EXISTS continuity_findings (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    scope            TEXT NOT NULL,
    scope_key        TEXT NOT NULL,
    entity           TEXT NOT NULL,
    attribute        TEXT NOT NULL,
    verdict          TEXT NOT NULL,   -- 'contradiction' | 'possible' | 'likely_intentional'
    confidence       INTEGER NOT NULL,
    explanation      TEXT NOT NULL,
    occurrences_json TEXT NOT NULL,
    generated_at     TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_continuity_findings_scope ON continuity_findings(scope, scope_key);

-- Prompt template system: editable AI prompts stored in DB instead of hardcoded.
-- Each report function has a system prompt and a user prompt template with {placeholders}.
CREATE TABLE IF NOT EXISTS prompt_templates (
    id              TEXT PRIMARY KEY,    -- e.g. 'continuity_extract', 'sdt_check'
    label           TEXT NOT NULL,
    system_prompt   TEXT NOT NULL,
    user_template   TEXT NOT NULL,       -- uses {chapter_title}, {chapter_text}, {bible}, etc.
    max_tokens      INTEGER NOT NULL DEFAULT 4000,
    json_mode       INTEGER NOT NULL DEFAULT 0,  -- 1 = force JSON response format
    version         INTEGER NOT NULL DEFAULT 1,
    updated_at      TEXT NOT NULL DEFAULT ''
);

-- Preprocessed chapter text, cached per report type. Invalidated when
-- the source file changes (via modified_at timestamp comparison).
CREATE TABLE IF NOT EXISTS preprocessed_chapters (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    story_folder   TEXT NOT NULL,
    chapter_file   TEXT NOT NULL,
    report_type    TEXT NOT NULL,        -- e.g. 'continuity_extract', 'sdt_check'
    processed_text TEXT NOT NULL,
    source_modified_at TEXT NOT NULL,    -- file mtime when preprocessed
    created_at     TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_preproc_unique ON preprocessed_chapters(story_folder, chapter_file, report_type);

-- Configurable story folder layout (scaffolded on Create empty story).
-- role: '' | 'manuscript' | 'bible' | 'characters'
CREATE TABLE IF NOT EXISTS folder_structure (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    path       TEXT NOT NULL,
    role       TEXT NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL DEFAULT 0
);

-- Ad campaign tracking (Marketing mode)
CREATE TABLE IF NOT EXISTS ad_platform_accounts (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    platform       TEXT NOT NULL,
    account_id     TEXT NOT NULL DEFAULT '',
    pixel_id       TEXT NOT NULL DEFAULT '',
    tracking_notes TEXT NOT NULL DEFAULT '',
    payment_notes  TEXT NOT NULL DEFAULT '',
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS ad_landing_pages (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    story_folder    TEXT NOT NULL,
    name            TEXT NOT NULL,
    url             TEXT NOT NULL DEFAULT '',
    conversion_rate REAL,
    notes           TEXT NOT NULL DEFAULT '',
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ad_landing_pages_story ON ad_landing_pages(story_folder);

CREATE TABLE IF NOT EXISTS ad_campaigns (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    story_folder        TEXT NOT NULL,
    name                TEXT NOT NULL,
    platform            TEXT NOT NULL DEFAULT '',
    platform_account_id INTEGER REFERENCES ad_platform_accounts(id),
    objective           TEXT NOT NULL DEFAULT 'awareness',
    status              TEXT NOT NULL DEFAULT 'draft',
    budget              REAL,
    budget_period       TEXT NOT NULL DEFAULT 'lifetime',
    start_date          TEXT NOT NULL DEFAULT '',
    end_date            TEXT NOT NULL DEFAULT '',
    target_audience     TEXT NOT NULL DEFAULT '',
    landing_page_id     INTEGER REFERENCES ad_landing_pages(id),
    notes               TEXT NOT NULL DEFAULT '',
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ad_campaigns_story ON ad_campaigns(story_folder);

CREATE TABLE IF NOT EXISTS ad_creatives (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    campaign_id     INTEGER NOT NULL REFERENCES ad_campaigns(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    creative_type   TEXT NOT NULL DEFAULT 'video',
    version         TEXT NOT NULL DEFAULT 'v1',
    platform_format TEXT NOT NULL DEFAULT '',
    status          TEXT NOT NULL DEFAULT 'draft',
    asset_path      TEXT NOT NULL DEFAULT '',
    body_text       TEXT NOT NULL DEFAULT '',
    notes           TEXT NOT NULL DEFAULT '',
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ad_creatives_campaign ON ad_creatives(campaign_id);

CREATE TABLE IF NOT EXISTS ad_performance_snapshots (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    campaign_id   INTEGER NOT NULL REFERENCES ad_campaigns(id) ON DELETE CASCADE,
    creative_id   INTEGER REFERENCES ad_creatives(id) ON DELETE SET NULL,
    snapshot_date TEXT NOT NULL,
    impressions   INTEGER NOT NULL DEFAULT 0,
    clicks        INTEGER NOT NULL DEFAULT 0,
    conversions   INTEGER NOT NULL DEFAULT 0,
    ctr           REAL NOT NULL DEFAULT 0,
    cpc           REAL NOT NULL DEFAULT 0,
    cpa           REAL NOT NULL DEFAULT 0,
    spend         REAL NOT NULL DEFAULT 0,
    notes         TEXT NOT NULL DEFAULT '',
    created_at    TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ad_perf_campaign_date ON ad_performance_snapshots(campaign_id, snapshot_date);

CREATE TABLE IF NOT EXISTS ad_spend_entries (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    campaign_id INTEGER NOT NULL REFERENCES ad_campaigns(id) ON DELETE CASCADE,
    platform    TEXT NOT NULL DEFAULT '',
    amount      REAL NOT NULL DEFAULT 0,
    spent_at    TEXT NOT NULL,
    notes       TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ad_spend_campaign ON ad_spend_entries(campaign_id);

CREATE TABLE IF NOT EXISTS ad_audience_notes (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    campaign_id     INTEGER NOT NULL REFERENCES ad_campaigns(id) ON DELETE CASCADE,
    label           TEXT NOT NULL DEFAULT '',
    demographics    TEXT NOT NULL DEFAULT '',
    interests       TEXT NOT NULL DEFAULT '',
    lookalike_notes TEXT NOT NULL DEFAULT '',
    outcome         TEXT NOT NULL DEFAULT 'untested',
    notes           TEXT NOT NULL DEFAULT '',
    created_at      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ad_audience_campaign ON ad_audience_notes(campaign_id);
"#;

pub struct Db(pub Mutex<Connection>);

/// Open (or create) the app's SQLite database in the platform app-data
/// directory, apply schema, and seed from JSON on first run only.
pub fn init(app: &AppHandle) -> Result<Db, String> {
    let dir: PathBuf = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Cannot create app data dir: {}", e))?;

    let db_path = dir.join("manuscript-intel.db");
    let conn = Connection::open(&db_path).map_err(|e| format!("Cannot open database: {}", e))?;
    conn.execute_batch(SCHEMA).map_err(|e| format!("Schema error: {}", e))?;

    // Migration: bisac_classifications gained a `format` column (ebook/print)
    // after initial release. Ignore the error if it already exists.
    let _ = conn.execute("ALTER TABLE bisac_classifications ADD COLUMN format TEXT NOT NULL DEFAULT 'ebook'", []);

    // Migration: kdp_categories gained `last_seen_at` so re-importing an
    // updated WinningCat file can detect categories that dropped out of the
    // new file (retired/renamed by Amazon) instead of leaving stale rows
    // sitting in the catalog forever with no signal they're outdated.
    let _ = conn.execute("ALTER TABLE kdp_categories ADD COLUMN last_seen_at TEXT", []);

    // Migration: chapter_summaries gained source_hash so summaries can be
    // regenerated only when normalized chapter content actually changes.
    let _ = conn.execute("ALTER TABLE chapter_summaries ADD COLUMN source_hash TEXT NOT NULL DEFAULT ''", []);

    // Migration: saved_reports table for versioned report storage.
    let _ = conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS saved_reports (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            story_folder TEXT NOT NULL,
            doc_type     TEXT NOT NULL,
            version      INTEGER NOT NULL,
            label        TEXT NOT NULL,
            content      TEXT NOT NULL,
            saved_at     TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_saved_reports_folder ON saved_reports(story_folder, doc_type);"
    );

    // Migration: move data from old pr_keywords table into mi_search_terms.
    // The schema already creates mi_search_terms, so we just copy any data and drop the old table.
    let _ = conn.execute_batch(
        "INSERT OR IGNORE INTO mi_search_terms (story_folder, generated_at, keywords_json)
         SELECT story_folder, generated_at, keywords_json FROM pr_keywords;
         DROP TABLE IF EXISTS pr_keywords;"
    );

    // Migration: story_documents from single-version (PRIMARY KEY on story_folder+doc_type)
    // to multi-version (auto-increment id). Recreate table if it lacks an id column.
    let has_id: bool = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('story_documents') WHERE name = 'id'",
        [], |r| r.get::<_, i64>(0)
    ).unwrap_or(0) > 0;

    if !has_id {
        let _ = conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS story_documents_new (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                story_folder TEXT NOT NULL,
                doc_type     TEXT NOT NULL,
                content      TEXT NOT NULL,
                generated_at TEXT NOT NULL
            );
            INSERT INTO story_documents_new (story_folder, doc_type, content, generated_at)
                SELECT story_folder, doc_type, content, generated_at FROM story_documents;
            DROP TABLE story_documents;
            ALTER TABLE story_documents_new RENAME TO story_documents;
            CREATE INDEX IF NOT EXISTS idx_story_docs_folder ON story_documents(story_folder, doc_type);"
        );
    }

    // Migration: series.bible_path column for series bible support.
    let _ = conn.execute("ALTER TABLE series ADD COLUMN bible_path TEXT NOT NULL DEFAULT ''", []);

    // Migration: cost / model metadata on report_types (dev DBs created before these columns).
    for col_def in [
        "ALTER TABLE report_types ADD COLUMN cost_truncation INTEGER NOT NULL DEFAULT 4000",
        "ALTER TABLE report_types ADD COLUMN cost_output_max INTEGER NOT NULL DEFAULT 1000",
        "ALTER TABLE report_types ADD COLUMN cost_per_chapter INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE report_types ADD COLUMN cost_fixed_calls INTEGER NOT NULL DEFAULT 1",
        "ALTER TABLE report_types ADD COLUMN model_slot TEXT NOT NULL DEFAULT 'default'",
        "ALTER TABLE report_types ADD COLUMN min_tier TEXT NOT NULL DEFAULT 'basic'",
    ] {
        let _ = conn.execute(col_def, []);
    }

    let _ = conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS provider_models (
            id           TEXT PRIMARY KEY,
            provider     TEXT NOT NULL,
            owned_by     TEXT NOT NULL DEFAULT '',
            input_price  REAL,
            output_price REAL,
            sort_order   INTEGER NOT NULL DEFAULT 0
        );
         CREATE TABLE IF NOT EXISTS lookup_config (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );"
    );

    seed_if_empty(&conn)?;
    seed_bisac_if_empty(&conn)?;
    seed_report_types(&conn)?;
    seed_prompt_templates(&conn)?;
    seed_zeigarnik_config_if_empty(&conn)?;
    seed_provider_models(&conn)?;
    seed_lookup_config(&conn)?;
    migrate_legacy_summaries_to_fingerprints(&conn)?;
    invalidate_incomplete_fingerprints(&conn)?;
    migrate_artifact_architecture(&conn)?;
    backfill_books_kdp_catalog(&conn)?;

    Ok(Db(Mutex::new(conn)))
}

fn seed_if_empty(conn: &Connection) -> Result<(), String> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM genres", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if count > 0 { return Ok(()); }

    #[derive(serde::Deserialize)]
    struct SeedGenre { name: String, description: String }

    let genres: Vec<SeedGenre> = serde_json::from_str(SEED_GENRE_LIST_JSON)
        .map_err(|e| format!("Cannot parse seed genre-list.json: {}", e))?;
    let kdp_map: std::collections::HashMap<String, Vec<String>> =
        serde_json::from_str(SEED_GENRE_KDP_MAP_JSON)
            .map_err(|e| format!("Cannot parse seed genre-kdp-map.json: {}", e))?;

    let now = chrono::Utc::now().to_rfc3339();

    for g in &genres {
        conn.execute(
            "INSERT OR IGNORE INTO genres (name, description) VALUES (?1, ?2)",
            params![g.name, g.description],
        ).map_err(|e| e.to_string())?;

        let genre_id: i64 = conn.query_row(
            "SELECT id FROM genres WHERE name = ?1", params![g.name], |r| r.get(0)
        ).map_err(|e| e.to_string())?;

        if let Some(paths) = kdp_map.get(&g.name) {
            for path in paths {
                for store in ["Kindle", "Books"] {
                    conn.execute(
                        "INSERT OR IGNORE INTO kdp_categories (path, store, source, created_at)
                         VALUES (?1, ?2, 'manual', ?3)",
                        params![path, store, now],
                    ).map_err(|e| e.to_string())?;

                    let category_id: i64 = conn.query_row(
                        "SELECT id FROM kdp_categories WHERE path = ?1 AND store = ?2",
                        params![path, store], |r| r.get(0)
                    ).map_err(|e| e.to_string())?;

                    conn.execute(
                        "INSERT OR IGNORE INTO genre_kdp_links (genre_id, category_id) VALUES (?1, ?2)",
                        params![genre_id, category_id],
                    ).map_err(|e| e.to_string())?;
                }
            }
        }
    }

    Ok(())
}

/// Mirror Kindle catalog paths into the Books (print) store when missing.
/// Print browse paths often share the same text as Kindle; node IDs differ and
/// are filled in on a full WinningCat re-import. This unblocks paperback
/// category matching for DBs that only had Kindle rows.
fn backfill_books_kdp_catalog(conn: &Connection) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR IGNORE INTO kdp_categories (path, store, source, created_at)
         SELECT path, 'Books', 'print_path_mirror', ?1
         FROM kdp_categories
         WHERE store = 'Kindle'",
        params![now],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR IGNORE INTO genre_kdp_links (genre_id, category_id)
         SELECT gkl.genre_id, kc_books.id
         FROM genre_kdp_links gkl
         JOIN kdp_categories kc_kindle ON kc_kindle.id = gkl.category_id AND kc_kindle.store = 'Kindle'
         JOIN kdp_categories kc_books ON kc_books.path = kc_kindle.path AND kc_books.store = 'Books'",
        [],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

fn seed_bisac_if_empty(conn: &Connection) -> Result<(), String> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM bisac_codes", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if count > 0 { return Ok(()); }

    #[derive(serde::Deserialize)]
    struct SeedBisac { code: String, heading: String }

    let codes: Vec<SeedBisac> = serde_json::from_str(SEED_BISAC_JSON)
        .map_err(|e| format!("Cannot parse seed bisac-fiction.json: {}", e))?;

    for c in &codes {
        conn.execute(
            "INSERT OR IGNORE INTO bisac_codes (code, heading) VALUES (?1, ?2)",
            params![c.code, c.heading],
        ).map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn seed_report_types(conn: &Connection) -> Result<(), String> {
    // (id, label, description, platforms, depends_on,
    //  cost_truncation, cost_output_max, cost_per_chapter, cost_fixed_calls, model_slot, min_tier)
    let rows: &[(&str, &str, &str, &str, &str, i64, i64, i64, i64, &str, &str)] = &[
        ("chapter_summaries", "Chapter Fingerprints", "Deterministic genre signal scan of every manuscript chapter (no AI).", "kdp,wide", "", 0, 0, 1, 0, "summaries", "basic"),
        ("genre_analysis", "Genre Analysis", "Industry genre classification, KDP paths, comps, and reader demographic.", "kdp,wide", "chapter_summaries", 0, 1200, 0, 1, "genre", "capable"),
        ("genre_ranking", "Genre Ranking", "Score the manuscript against all known genres independently.", "kdp,wide", "chapter_summaries,genre_analysis", 0, 1200, 0, 1, "genre", "capable"),
        ("kdp_categories", "KDP Categories", "Find the best-fit Amazon categories with discoverability stats.", "kdp", "chapter_summaries,genre_analysis,genre_ranking", 0, 1200, 0, 2, "keywords", "basic"),
        ("kdp_keywords", "KDP Keywords", "Optimize the 7 keyword strings for KDP discoverability.", "kdp", "chapter_summaries,genre_analysis,genre_ranking", 0, 1200, 0, 1, "keywords", "basic"),
        ("bisac_classification", "BISAC Classification", "Select BISAC subject codes for Ingram, wide distributors, and print metadata.", "wide", "chapter_summaries,genre_analysis", 0, 1200, 0, 2, "keywords", "basic"),
        ("mi_search_terms", "Search Terms", "Generate competition search phrases for market analysis.", "kdp", "chapter_summaries,genre_analysis", 0, 300, 0, 1, "keywords", "basic"),
        ("discovery_keywords", "Discovery Keywords", "Keywords optimized for Apple Books, Kobo, Google Play, and SEO.", "wide", "chapter_summaries,genre_analysis", 0, 1200, 0, 1, "keywords", "basic"),
        ("analysis", "Full Analysis", "Combined report: categories, keywords, and positioning all in one.", "kdp", "chapter_summaries,genre_analysis,genre_ranking,kdp_categories,kdp_keywords,mi_search_terms", 4000, 1000, 0, 1, "default", "basic"),
        ("keyword_search", "Keyword Search Results", "Amazon keyword volume and competition data from DataForSEO.", "kdp", "chapter_summaries,genre_analysis,genre_ranking", 4000, 1000, 0, 1, "keywords", "basic"),
        ("competition_report", "Competition Analysis", "Market landscape: how competitive the niche is, who dominates.", "kdp", "mi_search_terms", 4000, 1000, 0, 1, "default", "basic"),
        ("review_mining", "Reader Review Intelligence", "Reader insights extracted from competitor book reviews.", "kdp", "mi_search_terms", 4000, 1000, 0, 1, "default", "basic"),
        ("author_analysis", "Competitor Author Analysis", "Competitor pricing, release cadence, and series strategy.", "kdp", "mi_search_terms", 4000, 1000, 0, 1, "default", "basic"),
        ("zeigarnik_analysis", "Zeigarnik Effect", "Analyzes open loops and unresolved tension to maintain reader engagement.", "craft", "", 0, 0, 0, 0, "default", "basic"),
        ("continuity_check", "Continuity Check", "AI-assisted scan for contradicted facts — within a manuscript or across a whole series.", "craft", "", 6000, 4000, 1, 3, "continuity", "capable"),
        ("show_dont_tell", "Show Don't Tell", "AI-assisted check for telling instead of showing — flags violations with surrounding manuscript text.", "craft", "", 4000, 4000, 1, 0, "showDontTell", "capable"),
        ("ai_isms", "AI-isms", "AI-assisted check for prose habits that often read as machine-generated — flags passages with surrounding manuscript text.", "craft", "", 4000, 4000, 1, 0, "aiIsms", "capable"),
        // StoryAuditor craft audits
        ("chekhovs_gun", "Chekhov's Gun", "Finds early significant elements and checks they pay off later.", "craft", "", 0, 4000, 0, 1, "continuity", "capable"),
        ("red_herring_vs_abandoned", "Red Herring vs Abandoned", "Separates intentional misdirection from dropped plot threads.", "craft", "", 0, 4000, 0, 1, "continuity", "capable"),
        ("foreshadowing_twist_fairness", "Foreshadowing & Twist Fairness", "Checks foreshadowing distribution and twist fairness.", "craft", "", 0, 4000, 0, 1, "continuity", "capable"),
        ("macguffin_clarity", "MacGuffin Clarity", "Checks the driving object or goal is clear and motivating.", "craft", "", 0, 4000, 0, 1, "continuity", "capable"),
        ("want_vs_need", "Want vs Need", "External want vs internal need and character growth.", "craft", "", 0, 4000, 0, 1, "continuity", "capable"),
        ("thematic_throughline", "Thematic Throughline", "Theme consistency across scenes, subplots, and arcs.", "craft", "", 0, 4000, 0, 1, "continuity", "capable"),
        ("mirror_foil_character", "Mirror/Foil Characters", "Reflect/contrast pairings and thematic payoff.", "craft", "", 0, 4000, 0, 1, "continuity", "capable"),
        ("pov_discipline", "POV Discipline", "POV shifts, head-hopping, and information leaks.", "craft", "", 0, 4000, 0, 1, "continuity", "capable"),
        ("story_beat_placement", "Story Beat Placement", "Beat timing vs story frameworks — early, late, or missing.", "craft", "", 0, 4000, 0, 1, "continuity", "capable"),
        ("scene_sequel_balance", "Scene/Sequel Balance", "Action scene vs reflective sequel ratio.", "craft", "", 0, 4000, 0, 1, "continuity", "capable"),
        ("timeline_flashback", "Timeline / Flashback", "Timeline and flashback clarity and purpose.", "craft", "", 0, 4000, 0, 1, "continuity", "capable"),
        ("dramatic_irony", "Dramatic Irony", "Reader-knows-more moments — tension, humor, or dread.", "craft", "", 0, 4000, 0, 1, "continuity", "capable"),
        ("stakes_escalation", "Stakes Escalation", "Rising stakes across the arc; plateaus and reversals.", "craft", "", 0, 4000, 0, 1, "continuity", "capable"),
        ("cross_book_setup_payoff", "Cross-Book Setup/Payoff", "Series setups planted earlier that must pay off later.", "craft", "", 0, 4000, 0, 1, "continuity", "capable"),
        ("series_pacing_comparator", "Series Pacing Comparator", "Pacing curves compared across books in a series.", "craft", "", 0, 4000, 0, 1, "continuity", "capable"),
        ("recurring_motif_theme_series", "Recurring Motif/Theme (Series)", "Motifs and themes tracked across the series.", "craft", "", 0, 4000, 0, 1, "continuity", "capable"),
        // Publish platform (StoryAuditor marketing features)
        ("ai_beta_reader", "AI Beta Reader", "Chapter-by-chapter reader reactions and put-it-down risk.", "publish", "", 4000, 1200, 1, 0, "prose", "strong"),
        ("cliffhanger_score", "Cliffhanger Score", "How hard each chapter ending pulls into the next.", "publish", "", 4000, 500, 1, 0, "summaries", "basic"),
        ("hook_strength", "Hook Strength", "Would a browsing reader keep going past page one?", "publish", "", 0, 1200, 0, 1, "summaries", "basic"),
        ("pacing_curve", "Pacing Curve", "Where the story drags — per-chapter pace scores.", "publish", "", 4000, 600, 1, 0, "summaries", "basic"),
        ("blurb_builder", "Blurb Builder", "Back-cover / Amazon description variants plus short-form and BookBub one-liners.", "publish", "", 0, 3000, 0, 1, "prose", "strong"),
        ("line_polish", "Line-level Polish", "Filter words, echoes, adverbs, and passive voice (heuristic).", "publish", "", 0, 0, 0, 0, "default", "basic"),
        ("vellum_prep", "Vellum & Atticus Prep", "Clean manuscript export for formatter import.", "publish", "", 0, 0, 0, 0, "default", "basic"),
    ];

    for (id, label, description, platforms, depends_on, trunc, out_max, per_ch, fixed, slot, tier) in rows {
        conn.execute(
            "INSERT INTO report_types (
                id, label, description, platforms, depends_on,
                cost_truncation, cost_output_max, cost_per_chapter, cost_fixed_calls, model_slot, min_tier
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(id) DO UPDATE SET
                label = excluded.label,
                description = excluded.description,
                platforms = excluded.platforms,
                depends_on = excluded.depends_on,
                cost_truncation = excluded.cost_truncation,
                cost_output_max = excluded.cost_output_max,
                cost_per_chapter = excluded.cost_per_chapter,
                cost_fixed_calls = excluded.cost_fixed_calls,
                model_slot = excluded.model_slot,
                min_tier = excluded.min_tier",
            params![id, label, description, platforms, depends_on, trunc, out_max, per_ch, fixed, slot, tier],
        ).map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn seed_prompt_templates(conn: &Connection) -> Result<(), String> {
    #[derive(serde::Deserialize)]
    struct SeedPrompt {
        id: String,
        label: String,
        system_prompt: String,
        user_template: String,
        max_tokens: i64,
        json_mode: i64,
    }

    let templates: Vec<SeedPrompt> = serde_json::from_str(SEED_PROMPT_TEMPLATES_JSON)
        .map_err(|e| format!("Cannot parse seed prompt-templates.json: {}", e))?;

    // Dev app: always refresh from seed so prompt edits ship with the build.
    conn.execute("DELETE FROM prompt_templates", [])
        .map_err(|e| e.to_string())?;

    for t in &templates {
        conn.execute(
            "INSERT INTO prompt_templates (id, label, system_prompt, user_template, max_tokens, json_mode, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))",
            params![t.id, t.label, t.system_prompt, t.user_template, t.max_tokens, t.json_mode],
        ).map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn seed_provider_models(conn: &Connection) -> Result<(), String> {
    #[derive(serde::Deserialize)]
    struct SeedModel {
        id: String,
        provider: String,
        owned_by: String,
        input_price: Option<f64>,
        output_price: Option<f64>,
        sort_order: i64,
    }

    let models: Vec<SeedModel> = serde_json::from_str(SEED_PROVIDER_MODELS_JSON)
        .map_err(|e| format!("Cannot parse seed provider-models.json: {}", e))?;

    conn.execute("DELETE FROM provider_models", [])
        .map_err(|e| e.to_string())?;

    for m in &models {
        conn.execute(
            "INSERT INTO provider_models (id, provider, owned_by, input_price, output_price, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![m.id, m.provider, m.owned_by, m.input_price, m.output_price, m.sort_order],
        ).map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn seed_lookup_config(conn: &Connection) -> Result<(), String> {
    let parsed: serde_json::Value = serde_json::from_str(SEED_LOOKUP_CONFIG_JSON)
        .map_err(|e| format!("Cannot parse seed lookup-config.json: {}", e))?;
    let obj = parsed.as_object().ok_or("lookup-config.json must be a JSON object")?;

    conn.execute("DELETE FROM lookup_config", [])
        .map_err(|e| e.to_string())?;

    for (key, value) in obj {
        conn.execute(
            "INSERT INTO lookup_config (key, value) VALUES (?1, ?2)",
            params![key, value.to_string()],
        ).map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn seed_zeigarnik_config_if_empty(conn: &Connection) -> Result<(), String> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM zeigarnik_config", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if count > 0 { return Ok(()); }

    let parsed: serde_json::Value = serde_json::from_str(SEED_ZEIGARNIK_CONFIG_JSON)
        .map_err(|e| format!("Cannot parse seed zeigarnik-config.json: {}", e))?;

    let obj = parsed.as_object().ok_or("zeigarnik-config.json must be a JSON object")?;
    for (key, value) in obj {
        if key == "thresholds" {
            // Flatten thresholds into individual keys so each is independently tunable.
            if let Some(t) = value.as_object() {
                for (tkey, tval) in t {
                    conn.execute(
                        "INSERT OR IGNORE INTO zeigarnik_config (key, value) VALUES (?1, ?2)",
                        params![format!("threshold.{}", tkey), tval.to_string()],
                    ).map_err(|e| e.to_string())?;
                }
            }
        } else {
            conn.execute(
                "INSERT OR IGNORE INTO zeigarnik_config (key, value) VALUES (?1, ?2)",
                params![key, value.to_string()],
            ).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

// ── Zeigarnik effect detector (craft platform, no AI) ──────────────────────

#[derive(Clone, Debug)]
pub struct ZeigarnikConfig {
    pub cliffhanger_markers:  Vec<String>,
    pub resolution_markers:   Vec<String>,
    pub question_lead_ins:    Vec<String>,
    pub short_fragment_max_words:      usize,
    pub min_gap_chapters_for_thread:   usize,
    pub max_total_mentions_for_thread: usize,
    pub min_thread_term_len:           usize,
    pub top_threads_limit:             usize,
    pub min_question_words:            usize,
    pub max_questions_per_chapter:     usize,
}

impl Default for ZeigarnikConfig {
    fn default() -> Self {
        ZeigarnikConfig {
            cliffhanger_markers: vec![], resolution_markers: vec![], question_lead_ins: vec![],
            short_fragment_max_words: 8, min_gap_chapters_for_thread: 3,
            max_total_mentions_for_thread: 6, min_thread_term_len: 4,
            top_threads_limit: 25, min_question_words: 4, max_questions_per_chapter: 6,
        }
    }
}

/// Load the Zeigarnik phrase lists and thresholds from the database. Falls
/// back to sane defaults for any key missing (e.g. a fresh DB where seeding
/// somehow failed) rather than erroring the whole analysis out.
pub fn load_zeigarnik_config(conn: &Connection) -> ZeigarnikConfig {
    let mut cfg = ZeigarnikConfig::default();

    let get_str_list = |key: &str| -> Vec<String> {
        conn.query_row("SELECT value FROM zeigarnik_config WHERE key = ?1", params![key], |r| r.get::<_, String>(0))
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
            .unwrap_or_default()
    };
    let get_usize = |key: &str, default: usize| -> usize {
        conn.query_row("SELECT value FROM zeigarnik_config WHERE key = ?1", params![format!("threshold.{}", key)], |r| r.get::<_, String>(0))
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(default)
    };

    cfg.cliffhanger_markers = get_str_list("cliffhanger_markers");
    cfg.resolution_markers  = get_str_list("resolution_markers");
    cfg.question_lead_ins   = get_str_list("question_lead_ins");
    cfg.short_fragment_max_words      = get_usize("short_fragment_max_words", 8);
    cfg.min_gap_chapters_for_thread   = get_usize("min_gap_chapters_for_thread", 3);
    cfg.max_total_mentions_for_thread = get_usize("max_total_mentions_for_thread", 6);
    cfg.min_thread_term_len           = get_usize("min_thread_term_len", 4);
    cfg.top_threads_limit             = get_usize("top_threads_limit", 25);
    cfg.min_question_words            = get_usize("min_question_words", 4);
    cfg.max_questions_per_chapter     = get_usize("max_questions_per_chapter", 6);

    cfg
}

#[derive(Clone, Debug)]
pub struct ZeigarnikChapterRow {
    pub chapter_index:  i64,
    pub file:           String,
    pub title:          String,
    pub word_count:     i64,
    pub sentence_count: i64,
    pub question_count: i64,
    pub ending_type:    String,
    pub tension_score:  i64,
    pub ending_snippet: String,
}

#[derive(Clone, Debug)]
pub struct ZeigarnikThreadRow {
    pub term:                String,
    pub mention_count:       i64,
    pub first_chapter_index: i64,
    pub first_file:          String,
    pub first_snippet:       String,
    pub gap_start_index:     i64,
    pub gap_end_index:       i64,
    pub max_gap_chapters:    i64,
    pub max_gap_words:       i64,
}

/// Replace all stored Zeigarnik chapter metrics + threads for a story with a
/// fresh set — same "latest run supersedes" model used everywhere else.
pub fn replace_zeigarnik_analysis(
    conn: &Connection,
    story_folder: &str,
    chapters: &[ZeigarnikChapterRow],
    threads: &[ZeigarnikThreadRow],
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute("DELETE FROM zeigarnik_chapters WHERE story_folder = ?1", params![story_folder]).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM zeigarnik_threads WHERE story_folder = ?1", params![story_folder]).map_err(|e| e.to_string())?;

    for c in chapters {
        conn.execute(
            "INSERT INTO zeigarnik_chapters
             (story_folder, chapter_index, file, title, word_count, sentence_count, question_count, ending_type, tension_score, ending_snippet, generated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![story_folder, c.chapter_index, c.file, c.title, c.word_count, c.sentence_count, c.question_count, c.ending_type, c.tension_score, c.ending_snippet, now],
        ).map_err(|e| e.to_string())?;
    }

    for t in threads {
        conn.execute(
            "INSERT INTO zeigarnik_threads
             (story_folder, term, mention_count, first_chapter_index, first_file, first_snippet, gap_start_index, gap_end_index, max_gap_chapters, max_gap_words, generated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![story_folder, t.term, t.mention_count, t.first_chapter_index, t.first_file, t.first_snippet, t.gap_start_index, t.gap_end_index, t.max_gap_chapters, t.max_gap_words, now],
        ).map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn has_zeigarnik_analysis(conn: &Connection, story_folder: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM zeigarnik_chapters WHERE story_folder = ?1 LIMIT 1",
        params![story_folder], |_| Ok(())
    ).is_ok()
}

// ── Series (Continuity Checker: grouping stories in reading order) ────────────

#[derive(serde::Serialize, Clone, Debug)]
pub struct SeriesRow {
    pub id:         i64,
    pub name:       String,
    pub book_count: i64,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct SeriesBookRow {
    pub story_folder: String,
    pub story_name:   String,
    pub book_order:   i64,
}

pub fn list_series(conn: &Connection) -> Result<Vec<SeriesRow>, String> {
    let mut stmt = conn.prepare(
        "SELECT s.id, s.name, COUNT(sb.story_folder)
         FROM series s LEFT JOIN series_books sb ON sb.series_id = s.id
         GROUP BY s.id ORDER BY s.name"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| {
        Ok(SeriesRow { id: r.get(0)?, name: r.get(1)?, book_count: r.get(2)? })
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn create_series(conn: &Connection, name: &str) -> Result<SeriesRow, String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute("INSERT INTO series (name, created_at) VALUES (?1, ?2)", params![name.trim(), now])
        .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    Ok(SeriesRow { id, name: name.trim().to_string(), book_count: 0 })
}

/// Deletes the series and its book memberships. Does NOT delete the stories
/// themselves or any continuity data already recorded under the series key.
pub fn delete_series(conn: &Connection, series_id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM series_books WHERE series_id = ?1", params![series_id]).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM series WHERE id = ?1", params![series_id]).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn list_series_books(conn: &Connection, series_id: i64) -> Result<Vec<SeriesBookRow>, String> {
    let mut stmt = conn.prepare(
        "SELECT story_folder, story_name, book_order FROM series_books
         WHERE series_id = ?1 ORDER BY book_order"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![series_id], |r| {
        Ok(SeriesBookRow { story_folder: r.get(0)?, story_name: r.get(1)?, book_order: r.get(2)? })
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn add_story_to_series(conn: &Connection, series_id: i64, story_folder: &str, story_name: &str, book_order: i64) -> Result<(), String> {
    conn.execute(
        "INSERT INTO series_books (series_id, story_folder, story_name, book_order) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(series_id, story_folder) DO UPDATE SET story_name = excluded.story_name, book_order = excluded.book_order",
        params![series_id, story_folder, story_name, book_order],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn remove_story_from_series(conn: &Connection, series_id: i64, story_folder: &str) -> Result<(), String> {
    conn.execute("DELETE FROM series_books WHERE series_id = ?1 AND story_folder = ?2", params![series_id, story_folder])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn list_series_cmd(db: tauri::State<'_, Db>) -> Result<Vec<SeriesRow>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    list_series(&conn)
}

#[tauri::command]
pub async fn create_series_cmd(db: tauri::State<'_, Db>, name: String) -> Result<SeriesRow, String> {
    if name.trim().is_empty() { return Err("Series name cannot be empty.".to_string()); }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    create_series(&conn, &name)
}

#[tauri::command]
pub async fn delete_series_cmd(db: tauri::State<'_, Db>, series_id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    delete_series(&conn, series_id)
}

#[tauri::command]
pub async fn list_series_books_cmd(db: tauri::State<'_, Db>, series_id: i64) -> Result<Vec<SeriesBookRow>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    list_series_books(&conn, series_id)
}

#[derive(serde::Deserialize)]
pub struct AddToSeriesRequest {
    pub series_id:    i64,
    pub story_folder: String,
    pub story_name:   String,
    pub book_order:   i64,
}

#[tauri::command]
pub async fn add_story_to_series_cmd(db: tauri::State<'_, Db>, request: AddToSeriesRequest) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    add_story_to_series(&conn, request.series_id, &request.story_folder, &request.story_name, request.book_order)
}

#[tauri::command]
pub async fn remove_story_from_series_cmd(db: tauri::State<'_, Db>, series_id: i64, story_folder: String) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    remove_story_from_series(&conn, series_id, &story_folder)
}

// ── Continuity Checker (craft platform, AI-assisted) ────────────────────

#[derive(Clone, Debug)]
pub struct ContinuityFactRow {
    pub chapter_index: i64,
    pub file:          String,
    pub chapter_title: String,
    pub entity:        String,
    pub entity_type:   String,
    pub attribute:     String,
    pub value:         String,
    pub snippet:       String,
}

pub fn replace_continuity_facts(conn: &Connection, story_folder: &str, facts: &[ContinuityFactRow]) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute("DELETE FROM continuity_facts WHERE story_folder = ?1", params![story_folder]).map_err(|e| e.to_string())?;
    for f in facts {
        conn.execute(
            "INSERT INTO continuity_facts
             (story_folder, chapter_index, file, chapter_title, entity, entity_type, attribute, value, snippet, generated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![story_folder, f.chapter_index, f.file, f.chapter_title, f.entity, f.entity_type, f.attribute, f.value, f.snippet, now],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct ContinuityOccurrence {
    pub story_folder:  String,
    pub story_name:    String,
    pub file:          String,
    pub chapter_title: String,
    pub chapter_index: i64,
    pub value:         String,
    pub snippet:       String,
}

#[derive(Clone, Debug)]
pub struct ContinuityFindingRow {
    pub entity:       String,
    pub attribute:    String,
    pub verdict:      String,
    pub confidence:   i64,
    pub explanation:  String,
    pub occurrences:  Vec<ContinuityOccurrence>,
}

/// Replace all stored findings for a scope (one manuscript, or one series) —
/// same "latest run supersedes" model used everywhere else.
pub fn replace_continuity_findings(conn: &Connection, scope: &str, scope_key: &str, findings: &[ContinuityFindingRow]) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute("DELETE FROM continuity_findings WHERE scope = ?1 AND scope_key = ?2", params![scope, scope_key]).map_err(|e| e.to_string())?;
    for f in findings {
        let occ_json = serde_json::to_string(&f.occurrences).unwrap_or_default();
        conn.execute(
            "INSERT INTO continuity_findings
             (scope, scope_key, entity, attribute, verdict, confidence, explanation, occurrences_json, generated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![scope, scope_key, f.entity, f.attribute, f.verdict, f.confidence, f.explanation, occ_json, now],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ── Tauri commands (for future UI — browsing/editing the genre/category map) ───────────

#[tauri::command]
pub async fn list_genres_cmd(db: tauri::State<'_, Db>) -> Result<Vec<GenreRow>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    list_genres(&conn)
}

#[derive(serde::Deserialize)]
pub struct AddKdpPathRequest {
    pub genre_name: String,
    pub path:       String,
    pub store:      String,
}

#[tauri::command]
pub async fn add_kdp_path_cmd(db: tauri::State<'_, Db>, request: AddKdpPathRequest) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    upsert_kdp_path(&conn, &request.genre_name, &request.path, &request.store, "manual", false)
}

// ── Query helpers used by genre_analyzer.rs / category_finder.rs ──────────────

#[derive(serde::Serialize, Clone, Debug)]
pub struct GenreRow {
    pub id:          i64,
    pub name:        String,
    pub description: String,
}

pub fn list_genres(conn: &Connection) -> Result<Vec<GenreRow>, String> {
    let mut stmt = conn.prepare("SELECT id, name, description FROM genres ORDER BY name")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| {
        Ok(GenreRow { id: r.get(0)?, name: r.get(1)?, description: r.get(2)? })
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Get every known KDP path for a genre name (by exact name match).
pub fn kdp_paths_for_genre(conn: &Connection, genre_name: &str, store: &str) -> Result<Vec<String>, String> {
    let mut stmt = conn.prepare(
        "SELECT kc.path FROM kdp_categories kc
         JOIN genre_kdp_links gkl ON gkl.category_id = kc.id
         JOIN genres g ON g.id = gkl.genre_id
         WHERE g.name = ?1 AND kc.store = ?2
         ORDER BY kc.verified_at DESC NULLS LAST"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![genre_name, store], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Record (or update) a KDP category path and link it to a genre. Used both
/// for manual corrections and for auto-growth from Category Finder results.
/// Marks the path as verified (sets verified_at) when `verified` is true —
/// i.e. when it came from a live, successful category lookup.
pub fn upsert_kdp_path(
    conn: &Connection,
    genre_name: &str,
    path: &str,
    store: &str,
    source: &str,
    verified: bool,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO kdp_categories (path, store, source, verified_at, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(path, store) DO UPDATE SET
            verified_at = CASE WHEN ?4 IS NOT NULL THEN ?4 ELSE kdp_categories.verified_at END,
            source = excluded.source",
        params![path, store, source, if verified { Some(now.clone()) } else { None::<String> }, now],
    ).map_err(|e| e.to_string())?;

    let category_id: i64 = conn.query_row(
        "SELECT id FROM kdp_categories WHERE path = ?1 AND store = ?2",
        params![path, store], |r| r.get(0)
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR IGNORE INTO genres (name, description) VALUES (?1, '')",
        params![genre_name],
    ).map_err(|e| e.to_string())?;

    let genre_id: i64 = conn.query_row(
        "SELECT id FROM genres WHERE name = ?1", params![genre_name], |r| r.get(0)
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR IGNORE INTO genre_kdp_links (genre_id, category_id) VALUES (?1, ?2)",
        params![genre_id, category_id],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

/// Replace all stored genre rankings for a story with a fresh set — "latest
/// ranking wins" rather than accumulating history, since re-running Rank
/// Genres means the previous ranking is superseded, not a separate data point.
pub fn replace_genre_rankings(
    conn: &Connection,
    story_folder: &str,
    rankings: &[(String, u8, String)],  // (genre_name, confidence, reason)
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute("DELETE FROM genre_rankings WHERE story_folder = ?1", params![story_folder])
        .map_err(|e| e.to_string())?;

    for (genre_name, confidence, reason) in rankings {
        conn.execute(
            "INSERT OR IGNORE INTO genres (name, description) VALUES (?1, '')",
            params![genre_name],
        ).map_err(|e| e.to_string())?;
        let genre_id: i64 = conn.query_row(
            "SELECT id FROM genres WHERE name = ?1", params![genre_name], |r| r.get(0)
        ).map_err(|e| e.to_string())?;

        conn.execute(
            "INSERT INTO genre_rankings (story_folder, genre_id, confidence, reason, generated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![story_folder, genre_id, confidence, reason, now],
        ).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct RankingRow {
    pub genre:      String,
    pub confidence: i64,
    pub reason:     String,
    pub kdp_paths:  Vec<String>,
}

pub fn get_genre_rankings(conn: &Connection, story_folder: &str, store: &str) -> Result<Vec<RankingRow>, String> {
    let mut stmt = conn.prepare(
        "SELECT g.name, gr.confidence, gr.reason
         FROM genre_rankings gr JOIN genres g ON g.id = gr.genre_id
         WHERE gr.story_folder = ?1
         ORDER BY gr.confidence DESC"
    ).map_err(|e| e.to_string())?;

    let rows: Vec<(String, i64, String)> = stmt.query_map(params![story_folder], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?))
    }).map_err(|e| e.to_string())?
      .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for (genre, confidence, reason) in rows {
        let kdp_paths = kdp_paths_for_genre(conn, &genre, store)?;
        out.push(RankingRow { genre, confidence, reason, kdp_paths });
    }
    Ok(out)
}

pub fn has_genre_rankings(conn: &Connection, story_folder: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM genre_rankings WHERE story_folder = ?1 LIMIT 1",
        params![story_folder], |_| Ok(())
    ).is_ok()
}

pub fn has_category_results(conn: &Connection, story_folder: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM category_results WHERE story_folder = ?1 LIMIT 1",
        params![story_folder], |_| Ok(())
    ).is_ok()
}

pub fn kdp_category_count(conn: &Connection, store: &str) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM kdp_categories WHERE store = ?1",
        params![store], |r| r.get(0)
    ).unwrap_or(0)
}

/// Keyword search over the imported category catalog — case-insensitive
/// substring match per term, deduplicated, capped at `limit`. This is the
/// direct replacement for Category Finder's live top-level scraping: once
/// the catalog is populated (WinningCat import, or prior discoveries), this
/// is a plain SQL query instead of scraping any external UI at all.
pub fn search_kdp_categories(conn: &Connection, store: &str, terms: &[String], limit: usize) -> Vec<(String, String)> {
    if terms.is_empty() { return Vec::new(); }
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();

    for term in terms {
        let cleaned = term.replace('%', " ").replace('_', " ");
        let cleaned = cleaned.trim();
        if cleaned.is_empty() { continue; }
        let pattern = format!("%{}%", cleaned);

        let mut stmt = match conn.prepare(
            "SELECT path, COALESCE(amazon_node_id,'') FROM kdp_categories
             WHERE store = ?1 AND path LIKE ?2 ESCAPE '\\' COLLATE NOCASE LIMIT 200"
        ) { Ok(s) => s, Err(_) => continue };

        let rows: Vec<(String, String)> = match stmt.query_map(params![store, pattern], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        }) {
            Ok(mapped) => mapped.flatten().collect(),
            Err(_) => continue,
        };

        for row in rows {
            if seen.insert(row.0.clone()) {
                out.push(row);
                if out.len() >= limit { return out; }
            }
        }
    }
    out
}

/// Import a category path + node ID from an external catalog (WinningCat)
/// without linking it to any genre yet — that happens later via Category
/// Finder discovery or manual mapping. Preserves the source label if a path
/// was already verified live (category_finder /
/// category_analyzer outrank a catalog import), but always refreshes the
/// node ID and last_seen_at since those are authoritative either way.
pub fn import_kdp_category(conn: &Connection, path: &str, store: &str, node_id: &str) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO kdp_categories (path, store, amazon_node_id, source, created_at, last_seen_at)
         VALUES (?1, ?2, ?3, 'winningcat', ?4, ?4)
         ON CONFLICT(path, store) DO UPDATE SET
            amazon_node_id = excluded.amazon_node_id,
            last_seen_at = ?4,
            source = CASE
                WHEN kdp_categories.source IN ('category_finder', 'category_analyzer')
                THEN kdp_categories.source ELSE 'winningcat' END",
        params![path, store, node_id, now],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

/// Every catalog entry sourced from WinningCat that was NOT touched by an
/// import run started at or after `since` — i.e. it was in a previous
/// WinningCat file but missing from the latest one. Doesn't delete anything
/// automatically (Amazon renaming a category and it genuinely disappearing
/// look identical from here); surfaces the list so a human decides.
pub fn stale_winningcat_paths(conn: &Connection, since: &str) -> Vec<(String, String)> {
    let mut stmt = match conn.prepare(
        "SELECT path, store FROM kdp_categories
         WHERE source = 'winningcat' AND (last_seen_at IS NULL OR last_seen_at < ?1)
         ORDER BY store, path"
    ) { Ok(s) => s, Err(_) => return Vec::new() };
    stmt.query_map(params![since], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
        .unwrap_or_default()
}

/// Remove every WinningCat-sourced catalog entry not seen since `since`.
/// Called only when the user explicitly confirms cleanup after reviewing
/// the stale count from an import — not automatic.
pub fn remove_stale_winningcat_paths(conn: &Connection, since: &str) -> Result<usize, String> {
    conn.execute(
        "DELETE FROM kdp_categories WHERE source = 'winningcat' AND (last_seen_at IS NULL OR last_seen_at < ?1)",
        params![since],
    ).map_err(|e| e.to_string())
}

/// Replace all stored category-finder results for a story with a fresh set.
/// Every matched/considered result also gets written into kdp_categories and
/// linked to the genre it was found under (when it clears 80%, marked
/// verified — this is how the genre->KDP map grows from real usage).
pub fn replace_category_results(
    conn: &Connection,
    story_folder: &str,
    store: &str,
    top_genre_hint: Option<&str>,
    results: &[(String, u8, String, String, String, String, String, Option<String>)],
    // (path, confidence, sales_to_one, sales_to_ten, publisher_pct, ku_pct, status, note)
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute("DELETE FROM category_results WHERE story_folder = ?1", params![story_folder])
        .map_err(|e| e.to_string())?;

    for (path, confidence, sales_to_one, sales_to_ten, publisher_pct, ku_pct, status, note) in results {
        let category_id: Option<i64> = if status != "failed" {
            let _ = conn.execute(
                "INSERT INTO kdp_categories (path, store, source, verified_at, created_at)
                 VALUES (?1, ?2, 'category_finder', ?3, ?3)
                 ON CONFLICT(path, store) DO UPDATE SET verified_at = ?3, source = 'category_finder'",
                params![path, store, now],
            );
            let id: Option<i64> = conn.query_row(
                "SELECT id FROM kdp_categories WHERE path = ?1 AND store = ?2",
                params![path, store], |r| r.get(0)
            ).ok();

            if let (Some(cat_id), Some(genre_name)) = (id, top_genre_hint) {
                let _ = conn.execute(
                    "INSERT OR IGNORE INTO genres (name, description) VALUES (?1, '')",
                    params![genre_name],
                );
                if let Ok(genre_id) = conn.query_row::<i64, _, _>(
                    "SELECT id FROM genres WHERE name = ?1", params![genre_name], |r| r.get(0)
                ) {
                    let _ = conn.execute(
                        "INSERT OR IGNORE INTO genre_kdp_links (genre_id, category_id) VALUES (?1, ?2)",
                        params![genre_id, cat_id],
                    );
                }
            }
            id
        } else {
            None
        };

        conn.execute(
            "INSERT INTO category_results
             (story_folder, category_id, raw_path, store, confidence, sales_to_one, sales_to_ten,
              publisher_pct, ku_pct, status, note, generated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![story_folder, category_id, path, store, confidence, sales_to_one, sales_to_ten,
                     publisher_pct, ku_pct, status, note, now],
        ).map_err(|e| e.to_string())?;
    }

    Ok(())
}

// ── Chapter fingerprints ─────────────────────────────────────────────────

#[derive(serde::Serialize, Clone, Debug)]
pub struct ChapterSummaryRow {
    pub file:       String,
    pub title:      String,
    pub signals:    String,
    pub word_count: i64,
}

fn migrate_legacy_summaries_to_fingerprints(conn: &Connection) -> Result<(), String> {
    use crate::analysis::chapter_stats::ChapterFingerprint;

    let mut stmt = conn
        .prepare(
            "SELECT story_folder, file, title, signals, source_hash, word_count, updated_at
             FROM chapter_summaries",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, String>(6)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    for (story_folder, file, title, signals, source_hash, word_count, updated_at) in rows {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM chapter_fingerprints WHERE story_folder = ?1 AND file = ?2",
                params![story_folder, file],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if exists > 0 {
            continue;
        }

        if let Some(fp) = ChapterFingerprint::from_storage(&signals) {
            let lexicon_json =
                serde_json::to_string(&fp.lexicon).unwrap_or_else(|_| "{}".to_string());
            conn.execute(
                "INSERT INTO chapter_fingerprints
                 (story_folder, file, title, source_hash, word_count, sentence_count,
                  paragraph_count, dialogue_pct, pov, tense, pacing, lexicon_json, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    story_folder,
                    file,
                    fp.title,
                    source_hash,
                    fp.word_count as i64,
                    fp.sentence_count as i64,
                    fp.paragraph_count as i64,
                    fp.dialogue_pct as i64,
                    fp.pov,
                    fp.tense,
                    fp.pacing,
                    lexicon_json,
                    updated_at,
                ],
            )
            .map_err(|e| e.to_string())?;
        } else if !signals.trim().is_empty() {
            // Legacy prose summary — keep a minimal row so the chapter stays tracked.
            conn.execute(
                "INSERT INTO chapter_fingerprints
                 (story_folder, file, title, source_hash, word_count, lexicon_json, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, '{}', ?6)",
                params![story_folder, file, title, source_hash, word_count, updated_at],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

/// Legacy AI-summary rows migrated without POV/tense must be rescanned.
fn invalidate_incomplete_fingerprints(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "UPDATE chapter_fingerprints SET source_hash = ''
         WHERE trim(pov) = '' OR trim(tense) = ''",
        [],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE chapter_summaries SET source_hash = ''
         WHERE EXISTS (
           SELECT 1 FROM chapter_fingerprints f
           WHERE f.story_folder = chapter_summaries.story_folder
             AND f.file = chapter_summaries.file
             AND (trim(f.pov) = '' OR trim(f.tense) = '')
         )",
        [],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// True when the chapter has a full deterministic fingerprint (not a legacy stub).
pub fn chapter_fingerprint_complete(
    conn: &Connection,
    story_folder: &str,
    file: &str,
) -> bool {
    conn.query_row(
        "SELECT pov, tense FROM chapter_fingerprints
         WHERE story_folder = ?1 AND file = ?2",
        params![story_folder, file],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
    )
    .map(|(pov, tense)| !pov.trim().is_empty() && !tense.trim().is_empty())
    .unwrap_or(false)
}

fn fingerprint_row_to_summary(
    file: String,
    title: String,
    word_count: i64,
    sentence_count: i64,
    paragraph_count: i64,
    dialogue_pct: i64,
    pov: String,
    tense: String,
    pacing: String,
    lexicon_json: String,
) -> ChapterSummaryRow {
    use crate::analysis::chapter_stats::ChapterFingerprint;
    use std::collections::HashMap;

    let lexicon: HashMap<String, u32> =
        serde_json::from_str(&lexicon_json).unwrap_or_default();
    let fp = ChapterFingerprint {
        schema: ChapterFingerprint::SCHEMA.to_string(),
        title: if title.is_empty() { file.clone() } else { title.clone() },
        word_count: word_count.max(0) as usize,
        sentence_count: sentence_count.max(0) as usize,
        paragraph_count: paragraph_count.max(0) as usize,
        dialogue_pct: dialogue_pct.max(0) as u32,
        pov,
        tense,
        pacing,
        lexicon,
    };
    ChapterSummaryRow {
        file,
        title,
        signals: fp.to_storage_json(),
        word_count,
    }
}

pub fn save_chapter_fingerprint(
    conn: &Connection,
    story_folder: &str,
    file: &str,
    fp: &crate::analysis::chapter_stats::ChapterFingerprint,
    source_hash: &str,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let lexicon_json = serde_json::to_string(&fp.lexicon).unwrap_or_else(|_| "{}".to_string());
    let signals_json = fp.to_storage_json();

    conn.execute(
        "INSERT INTO chapter_fingerprints
         (story_folder, file, title, source_hash, word_count, sentence_count, paragraph_count,
          dialogue_pct, pov, tense, pacing, lexicon_json, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
         ON CONFLICT(story_folder, file) DO UPDATE SET
            title = excluded.title,
            source_hash = excluded.source_hash,
            word_count = excluded.word_count,
            sentence_count = excluded.sentence_count,
            paragraph_count = excluded.paragraph_count,
            dialogue_pct = excluded.dialogue_pct,
            pov = excluded.pov,
            tense = excluded.tense,
            pacing = excluded.pacing,
            lexicon_json = excluded.lexicon_json,
            updated_at = excluded.updated_at",
        params![
            story_folder,
            file,
            fp.title,
            source_hash,
            fp.word_count as i64,
            fp.sentence_count as i64,
            fp.paragraph_count as i64,
            fp.dialogue_pct as i64,
            fp.pov,
            fp.tense,
            fp.pacing,
            lexicon_json,
            now,
        ],
    )
    .map_err(|e| e.to_string())?;

    // Keep legacy table in sync for any external tooling still reading it.
    conn.execute(
        "INSERT INTO chapter_summaries (story_folder, file, title, signals, source_hash, word_count, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(story_folder, file) DO UPDATE SET
            title = excluded.title, signals = excluded.signals,
            source_hash = excluded.source_hash,
            word_count = excluded.word_count, updated_at = excluded.updated_at",
        params![story_folder, file, fp.title, signals_json, source_hash, fp.word_count as i64, now],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn load_chapter_summaries(conn: &Connection, story_folder: &str) -> Vec<ChapterSummaryRow> {
    let mut stmt = match conn.prepare(
        "SELECT file, title, word_count, sentence_count, paragraph_count, dialogue_pct,
                pov, tense, pacing, lexicon_json
         FROM chapter_fingerprints
         WHERE story_folder = ?1
         ORDER BY file",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let from_fingerprints: Vec<ChapterSummaryRow> = stmt
        .query_map(params![story_folder], |r| {
            Ok(fingerprint_row_to_summary(
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
                r.get(8)?,
                r.get(9)?,
            ))
        })
        .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
        .unwrap_or_default();

    if !from_fingerprints.is_empty() {
        return from_fingerprints;
    }

    // Fallback: legacy rows not yet migrated (should be rare after init migration).
    let mut legacy = match conn.prepare(
        "SELECT file, title, signals, word_count FROM chapter_summaries
         WHERE story_folder = ?1 ORDER BY file",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    legacy
        .query_map(params![story_folder], |r| {
            let signals: String = r.get(2)?;
            Ok(ChapterSummaryRow {
                file: r.get(0)?,
                title: r.get(1)?,
                signals: signals.clone(),
                word_count: r.get(3)?,
            })
        })
        .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
        .unwrap_or_default()
}

pub fn chapter_summary_count(conn: &Connection, story_folder: &str) -> i64 {
    let fp_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM chapter_fingerprints WHERE story_folder = ?1",
            params![story_folder],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if fp_count > 0 {
        return fp_count;
    }
    conn.query_row(
        "SELECT COUNT(*) FROM chapter_summaries WHERE story_folder = ?1",
        params![story_folder],
        |r| r.get(0),
    )
    .unwrap_or(0)
}

pub fn load_chapter_summary_hashes(conn: &Connection, story_folder: &str) -> std::collections::HashMap<String, String> {
    let mut out: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    if let Ok(mut stmt) = conn.prepare(
        "SELECT file, source_hash FROM chapter_fingerprints WHERE story_folder = ?1",
    ) {
        if let Ok(rows) = stmt.query_map(params![story_folder], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        }) {
            for row in rows.flatten() {
                out.insert(row.0, row.1);
            }
        }
    }

    if out.is_empty() {
        if let Ok(mut stmt) = conn.prepare(
            "SELECT file, source_hash FROM chapter_summaries WHERE story_folder = ?1",
        ) {
            if let Ok(rows) = stmt.query_map(params![story_folder], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            }) {
                for row in rows.flatten() {
                    out.insert(row.0, row.1);
                }
            }
        }
    }

    out
}

pub fn delete_chapter_summaries(conn: &Connection, story_folder: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM chapter_fingerprints WHERE story_folder = ?1",
        params![story_folder],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM chapter_summaries WHERE story_folder = ?1",
        params![story_folder],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ── Genre classification (industry genre + KDP paths + comps + notes) ──────────────

#[derive(Clone, Debug)]

pub struct GenreDataRow {
    pub industry_ebook:     String,
    pub industry_print:     String,
    pub genre_signals:      String,
    pub reader_demographic: String,
    pub bookstore_shelving: String,
    pub kdp_ebook:          Vec<String>,
    pub kdp_print:          Vec<String>,
    pub comps_ebook:        Vec<String>,
    pub comps_print:        Vec<String>,
    pub marketing_notes:    Vec<String>,
}

#[allow(clippy::too_many_arguments)]
pub fn save_genre_data(
    conn: &Connection,
    story_folder: &str,
    industry_ebook: &str,
    industry_print: &str,
    genre_signals: &str,
    reader_demographic: &str,
    bookstore_shelving: &str,
    kdp_ebook: &[String],
    kdp_print: &[String],
    comps_ebook: &[String],
    comps_print: &[String],
    marketing_notes: &[String],
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO genre_data
         (story_folder, generated_at, industry_ebook, industry_print, genre_signals,
          reader_demographic, bookstore_shelving, kdp_ebook_json, kdp_print_json,
          comps_ebook_json, comps_print_json, marketing_notes_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(story_folder) DO UPDATE SET
            generated_at = excluded.generated_at,
            industry_ebook = excluded.industry_ebook,
            industry_print = excluded.industry_print,
            genre_signals = excluded.genre_signals,
            reader_demographic = excluded.reader_demographic,
            bookstore_shelving = excluded.bookstore_shelving,
            kdp_ebook_json = excluded.kdp_ebook_json,
            kdp_print_json = excluded.kdp_print_json,
            comps_ebook_json = excluded.comps_ebook_json,
            comps_print_json = excluded.comps_print_json,
            marketing_notes_json = excluded.marketing_notes_json",
        params![
            story_folder, now, industry_ebook, industry_print, genre_signals,
            reader_demographic, bookstore_shelving,
            serde_json::to_string(kdp_ebook).unwrap_or_default(),
            serde_json::to_string(kdp_print).unwrap_or_default(),
            serde_json::to_string(comps_ebook).unwrap_or_default(),
            serde_json::to_string(comps_print).unwrap_or_default(),
            serde_json::to_string(marketing_notes).unwrap_or_default(),
        ],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_genre_data(conn: &Connection, story_folder: &str) -> Option<GenreDataRow> {
    conn.query_row(
        "SELECT industry_ebook, industry_print, genre_signals, reader_demographic,
                bookstore_shelving, kdp_ebook_json, kdp_print_json, comps_ebook_json,
                comps_print_json, marketing_notes_json
         FROM genre_data WHERE story_folder = ?1",
        params![story_folder],
        |r| {
            let parse = |s: String| serde_json::from_str::<Vec<String>>(&s).unwrap_or_default();
            Ok(GenreDataRow {
                industry_ebook:     r.get(0)?,
                industry_print:     r.get(1)?,
                genre_signals:      r.get(2)?,
                reader_demographic: r.get(3)?,
                bookstore_shelving: r.get(4)?,
                kdp_ebook:          parse(r.get(5)?),
                kdp_print:          parse(r.get(6)?),
                comps_ebook:        parse(r.get(7)?),
                comps_print:        parse(r.get(8)?),
                marketing_notes:    parse(r.get(9)?),
            })
        },
    ).ok()
}

// ── KDP keywords (the 7 ready-to-paste strings) ──────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct KdpKeywordEntry {
    pub string:    String,
    pub chars:     i64,
    pub rationale: String,
}

pub fn save_kdp_keywords(
    conn: &Connection, story_folder: &str, keywords: &[KdpKeywordEntry], strategy: &str, source_note: &str,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO kdp_keywords (story_folder, generated_at, keywords_json, strategy, source_note)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(story_folder) DO UPDATE SET
            generated_at = excluded.generated_at, keywords_json = excluded.keywords_json,
            strategy = excluded.strategy, source_note = excluded.source_note",
        params![story_folder, now, serde_json::to_string(keywords).unwrap_or_default(), strategy, source_note],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_kdp_keywords(conn: &Connection, story_folder: &str) -> Option<(Vec<KdpKeywordEntry>, String, String)> {
    conn.query_row(
        "SELECT keywords_json, strategy, source_note FROM kdp_keywords WHERE story_folder = ?1",
        params![story_folder],
        |r| {
            let json: String = r.get(0)?;
            let strategy: Option<String> = r.get(1)?;
            let note: Option<String> = r.get(2)?;
            Ok((json, strategy.unwrap_or_default(), note.unwrap_or_default()))
        },
    ).ok().map(|(json, strategy, note)| {
        let keywords: Vec<KdpKeywordEntry> = serde_json::from_str(&json).unwrap_or_default();
        (keywords, strategy, note)
    })
}

// ── MI search-term keywords ─────────────────────────────────────────

pub fn save_mi_search_terms(conn: &Connection, story_folder: &str, keywords: &[String]) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO mi_search_terms (story_folder, generated_at, keywords_json)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(story_folder) DO UPDATE SET generated_at = excluded.generated_at, keywords_json = excluded.keywords_json",
        params![story_folder, now, serde_json::to_string(keywords).unwrap_or_default()],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_mi_search_terms(conn: &Connection, story_folder: &str) -> Vec<String> {
    conn.query_row(
        "SELECT keywords_json FROM mi_search_terms WHERE story_folder = ?1",
        params![story_folder], |r| r.get::<_, String>(0)
    ).ok()
     .and_then(|json| serde_json::from_str(&json).ok())
     .unwrap_or_default()
}

// ── Non-KDP discovery keywords (broader platforms: Apple Books, Kobo, etc.) ──

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct DiscoveryKeywordEntry {
    pub phrase:    String,
    pub rationale: String,
}

pub fn save_discovery_keywords(conn: &Connection, story_folder: &str, entries: &[DiscoveryKeywordEntry]) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO discovery_keywords (story_folder, generated_at, keywords_json)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(story_folder) DO UPDATE SET generated_at = excluded.generated_at, keywords_json = excluded.keywords_json",
        params![story_folder, now, serde_json::to_string(entries).unwrap_or_default()],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_discovery_keywords(conn: &Connection, story_folder: &str) -> Vec<DiscoveryKeywordEntry> {
    conn.query_row(
        "SELECT keywords_json FROM discovery_keywords WHERE story_folder = ?1",
        params![story_folder], |r| r.get::<_, String>(0)
    ).ok()
     .and_then(|json| serde_json::from_str(&json).ok())
     .unwrap_or_default()
}

pub fn has_keyword_search_results(conn: &Connection, story_folder: &str) -> bool {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM keyword_search_results WHERE story_folder = ?1)",
        params![story_folder],
        |r| r.get::<_, bool>(0),
    ).unwrap_or(false)
}

// ── Keyword Search results (real search volume / competition) ──

/// Replace all stored results for this story+seed — latest search wins, same
/// "supersede, don't accumulate" model used everywhere else in this app.
pub fn replace_keyword_search_results(
    conn: &Connection, story_folder: &str, seed: &str,
    rows: &[(String, String, String, String)],  // (keyword, searches, competition, earnings)
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "DELETE FROM keyword_search_results WHERE story_folder = ?1 AND seed = ?2",
        params![story_folder, seed],
    ).map_err(|e| e.to_string())?;
    for (keyword, searches, competition, earnings) in rows {
        conn.execute(
            "INSERT INTO keyword_search_results (story_folder, seed, keyword, searches, competition, earnings, generated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![story_folder, seed, keyword, searches, competition, earnings, now],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}


// ── Manuscript fingerprint + artifact state ──────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Freshness {
    Missing,
    Fresh,
    Stale,
}

impl Freshness {
    pub fn as_str(self) -> &'static str {
        match self {
            Freshness::Missing => "missing",
            Freshness::Fresh => "fresh",
            Freshness::Stale => "stale",
        }
    }
}

fn migrate_artifact_architecture(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS story_manuscript_state (
            story_folder TEXT PRIMARY KEY,
            manuscript_fingerprint TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS story_artifact_state (
            story_folder TEXT NOT NULL,
            artifact_type TEXT NOT NULL,
            built_from_manuscript_fingerprint TEXT,
            updated_at TEXT,
            PRIMARY KEY (story_folder, artifact_type)
        );",
    )
    .map_err(|e| e.to_string())?;

    let has_status: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('story_documents') WHERE name = 'status'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;

    if !has_status {
        for sql in [
            "ALTER TABLE story_documents ADD COLUMN status TEXT NOT NULL DEFAULT 'current'",
            "ALTER TABLE story_documents ADD COLUMN manuscript_fingerprint_at_save TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE story_documents ADD COLUMN archived_at TEXT",
            "ALTER TABLE story_documents ADD COLUMN archive_reason TEXT",
        ] {
            let _ = conn.execute(sql, []);
        }

        conn.execute_batch(
            "UPDATE story_documents SET status = 'archived', archived_at = generated_at, archive_reason = 'migration'
             WHERE id IN (
               SELECT sd.id FROM story_documents sd
               INNER JOIN (
                 SELECT story_folder, doc_type, MAX(id) AS max_id
                 FROM story_documents GROUP BY story_folder, doc_type
               ) latest ON sd.story_folder = latest.story_folder AND sd.doc_type = latest.doc_type
               WHERE sd.id != latest.max_id
             );",
        )
        .map_err(|e| e.to_string())?;
    }

    let _ = conn.execute_batch(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_story_docs_current
         ON story_documents(story_folder, doc_type) WHERE status = 'current';",
    );

    let _ = conn.execute(
        "ALTER TABLE report_types ADD COLUMN hidden INTEGER NOT NULL DEFAULT 0",
        [],
    );
    let _ = conn.execute(
        "UPDATE report_types SET hidden = 1 WHERE id = 'chapter_summaries'",
        [],
    );

    Ok(())
}

pub fn stored_manuscript_fingerprint(conn: &Connection, story_folder: &str) -> Option<String> {
    conn.query_row(
        "SELECT manuscript_fingerprint FROM story_manuscript_state WHERE story_folder = ?1",
        params![story_folder],
        |r| r.get(0),
    )
    .ok()
}

fn upsert_manuscript_state(
    conn: &Connection,
    story_folder: &str,
    fingerprint: &str,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO story_manuscript_state (story_folder, manuscript_fingerprint, updated_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(story_folder) DO UPDATE SET
            manuscript_fingerprint = excluded.manuscript_fingerprint,
            updated_at = excluded.updated_at",
        params![story_folder, fingerprint, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn archive_current_reports(
    conn: &Connection,
    story_folder: &str,
    reason: &str,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE story_documents SET status = 'archived', archived_at = ?1, archive_reason = ?2
         WHERE story_folder = ?3 AND status = 'current'",
        params![now, reason, story_folder],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn mark_artifacts_stale(conn: &Connection, story_folder: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM story_artifact_state WHERE story_folder = ?1",
        params![story_folder],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn on_manuscript_changed(
    conn: &Connection,
    story_folder: &str,
    new_fingerprint: &str,
) -> Result<(), String> {
    archive_current_reports(conn, story_folder, "manuscript_changed")?;
    mark_artifacts_stale(conn, story_folder)?;
    upsert_manuscript_state(conn, story_folder, new_fingerprint)
}

/// Returns true when the manuscript fingerprint changed (reports were archived).
pub fn sync_manuscript_state(
    conn: &Connection,
    story_folder: &str,
    current_fingerprint: &str,
) -> Result<bool, String> {
    let stored = stored_manuscript_fingerprint(conn, story_folder);
    match stored.as_deref() {
        Some(s) if s == current_fingerprint => Ok(false),
        Some(_) => {
            on_manuscript_changed(conn, story_folder, current_fingerprint)?;
            Ok(true)
        }
        None => {
            upsert_manuscript_state(conn, story_folder, current_fingerprint)?;
            Ok(false)
        }
    }
}

pub fn artifact_status(
    conn: &Connection,
    story_folder: &str,
    artifact_type: &str,
    current_fp: &str,
) -> Freshness {
    let built: Option<String> = conn
        .query_row(
            "SELECT built_from_manuscript_fingerprint FROM story_artifact_state
             WHERE story_folder = ?1 AND artifact_type = ?2",
            params![story_folder, artifact_type],
            |r| r.get(0),
        )
        .ok()
        .flatten()
        .filter(|s: &String| !s.is_empty());

    match built {
        None => Freshness::Missing,
        Some(ref b) if b == current_fp => Freshness::Fresh,
        _ => Freshness::Stale,
    }
}

pub fn record_artifact_built(
    conn: &Connection,
    story_folder: &str,
    artifact_type: &str,
    fingerprint: &str,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO story_artifact_state (story_folder, artifact_type, built_from_manuscript_fingerprint, updated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(story_folder, artifact_type) DO UPDATE SET
            built_from_manuscript_fingerprint = excluded.built_from_manuscript_fingerprint,
            updated_at = excluded.updated_at",
        params![story_folder, artifact_type, fingerprint, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn report_freshness_status(
    conn: &Connection,
    story_folder: &str,
    doc_type: &str,
    current_fp: &str,
) -> Freshness {
    let saved_fp: Option<String> = conn
        .query_row(
            "SELECT manuscript_fingerprint_at_save FROM story_documents
             WHERE story_folder = ?1 AND doc_type = ?2 AND status = 'current'",
            params![story_folder, doc_type],
            |r| r.get(0),
        )
        .ok()
        .filter(|s: &String| !s.is_empty());

    match saved_fp {
        None => Freshness::Missing,
        Some(ref fp) if fp == current_fp => Freshness::Fresh,
        _ => Freshness::Stale,
    }
}

pub fn should_skip_report_save(
    conn: &Connection,
    story_folder: &str,
    doc_type: &str,
    current_fp: &str,
) -> bool {
    report_freshness_status(conn, story_folder, doc_type, current_fp) == Freshness::Fresh
}

// ── Story documents (rendered markdown cache, read by the Reports panel) ─────

/// Look up the display label for a doc_type from the report_types table.
/// Falls back to the doc_type string itself if not found.
fn label_for_doc_type(conn: &Connection, doc_type: &str) -> String {
    conn.query_row(
        "SELECT label FROM report_types WHERE id = ?1",
        params![doc_type],
        |r| r.get::<_, String>(0),
    ).unwrap_or_else(|_| doc_type.to_string())
}

pub fn save_document(conn: &Connection, story_folder: &str, doc_type: &str, content: &str) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    save_document_at(conn, story_folder, doc_type, content, &now)
}

pub fn save_document_at(conn: &Connection, story_folder: &str, doc_type: &str, content: &str, timestamp: &str) -> Result<(), String> {
    let fp = stored_manuscript_fingerprint(conn, story_folder).unwrap_or_default();
    save_document_current(conn, story_folder, doc_type, content, timestamp, &fp)
}

pub fn save_document_current(
    conn: &Connection,
    story_folder: &str,
    doc_type: &str,
    content: &str,
    timestamp: &str,
    manuscript_fingerprint: &str,
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM story_documents WHERE story_folder = ?1 AND doc_type = ?2 AND status = 'current'",
        params![story_folder, doc_type],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO story_documents (story_folder, doc_type, content, generated_at, status, manuscript_fingerprint_at_save)
         VALUES (?1, ?2, ?3, ?4, 'current', ?5)",
        params![story_folder, doc_type, content, timestamp, manuscript_fingerprint],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_document(conn: &Connection, story_folder: &str, doc_type: &str) -> Option<String> {
    conn.query_row(
        "SELECT content FROM story_documents WHERE story_folder = ?1 AND doc_type = ?2 AND status = 'current' LIMIT 1",
        params![story_folder, doc_type],
        |r| r.get(0),
    )
    .ok()
}

/// Distinct doc_types that have a current saved document for this story.
pub fn list_existing_doc_types(conn: &Connection, story_folder: &str) -> Vec<String> {
    let mut stmt = match conn.prepare(
        "SELECT DISTINCT doc_type FROM story_documents WHERE story_folder = ?1 AND status = 'current'",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    stmt.query_map(params![story_folder], |r| r.get::<_, String>(0))
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct DocMeta {
    pub id:           i64,
    pub doc_type:     String,
    pub label:        String,
    pub generated_at: String,
}

/// Response envelope for get_report_cmd — tells the frontend what format to expect.
#[derive(serde::Serialize, Clone, Debug)]
pub struct ReportEnvelope {
    pub id:           i64,
    pub doc_type:     String,
    pub label:        String,
    pub format:       String,   // "json" | "markdown"
    pub content:      String,
    pub generated_at: String,
}

pub fn list_documents(conn: &Connection, story_folder: &str) -> Vec<DocMeta> {
    let mut stmt = match conn.prepare(
        "SELECT id, doc_type, generated_at FROM story_documents
         WHERE story_folder = ?1 AND status = 'current' ORDER BY generated_at DESC",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let rows: Vec<(i64, String, String)> = stmt.query_map(params![story_folder], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?))
    }).and_then(|rows| rows.collect::<Result<Vec<_>, _>>()).unwrap_or_default();

    rows.into_iter().map(|(id, doc_type, generated_at)| {
        let label = label_for_doc_type(conn, &doc_type);
        DocMeta { id, doc_type, label, generated_at }
    }).collect()
}

// ── Tauri commands for the Reports panel ────────────────────────────

#[derive(serde::Serialize, Clone, Debug)]
pub struct ReportTypeDef {
    pub id:          String,
    pub label:       String,
    pub description: String,
    pub platforms:   Vec<String>,
    pub depends_on:  Vec<String>,
    pub model_slot:  String,
    pub min_tier:    String,
}

#[derive(Clone, Debug)]
pub struct ReportCostParams {
    pub truncation:  usize,
    pub output_max:  usize,
    pub per_chapter: bool,
    pub fixed_calls: usize,
}

impl Default for ReportCostParams {
    fn default() -> Self {
        Self { truncation: 4000, output_max: 1000, per_chapter: false, fixed_calls: 1 }
    }
}

/// Load cost-estimate parameters for a report type. Falls back to defaults if missing.
pub fn load_report_cost_params(conn: &Connection, report_id: &str) -> ReportCostParams {
    conn.query_row(
        "SELECT cost_truncation, cost_output_max, cost_per_chapter, cost_fixed_calls
         FROM report_types WHERE id = ?1",
        params![report_id],
        |r| Ok(ReportCostParams {
            truncation:  r.get::<_, i64>(0)? as usize,
            output_max:  r.get::<_, i64>(1)? as usize,
            per_chapter: r.get::<_, i64>(2)? != 0,
            fixed_calls: r.get::<_, i64>(3)? as usize,
        }),
    ).unwrap_or_default()
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct ProviderModelRow {
    pub id: String,
    pub owned_by: String,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
}

pub fn list_provider_models(conn: &Connection, provider: &str) -> Vec<ProviderModelRow> {
    let mut stmt = match conn.prepare(
        "SELECT id, owned_by, input_price, output_price FROM provider_models
         WHERE provider = ?1 ORDER BY sort_order ASC, id ASC"
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    stmt.query_map(params![provider], |r| {
        Ok(ProviderModelRow {
            id: r.get(0)?,
            owned_by: r.get(1)?,
            input_price: r.get(2)?,
            output_price: r.get(3)?,
        })
    }).ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
}

/// Load a JSON string-array from lookup_config. Returns empty vec if missing/invalid.
pub fn load_lookup_string_list(conn: &Connection, key: &str) -> Vec<String> {
    conn.query_row(
        "SELECT value FROM lookup_config WHERE key = ?1",
        params![key],
        |r| r.get::<_, String>(0),
    )
    .ok()
    .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
    .unwrap_or_default()
}

#[tauri::command]
pub async fn list_report_types_cmd(db: tauri::State<'_, Db>) -> Result<Vec<ReportTypeDef>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, label, description, platforms, depends_on, model_slot, min_tier
         FROM report_types WHERE COALESCE(hidden, 0) = 0 ORDER BY rowid",
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([], |r| {
        Ok(ReportTypeDef {
            id:          r.get(0)?,
            label:       r.get(1)?,
            description: r.get(2)?,
            platforms:   r.get::<_, String>(3)?.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
            depends_on:  r.get::<_, String>(4)?.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
            model_slot:  r.get(5)?,
            min_tier:    r.get(6)?,
        })
    }).map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_reports_cmd(db: tauri::State<'_, Db>, folder: String) -> Result<Vec<DocMeta>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    Ok(list_documents(&conn, &folder))
}

#[tauri::command]
pub async fn save_activity_log_cmd(db: tauri::State<'_, Db>, folder: String, content: String, timestamp: String) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let ts = if timestamp.is_empty() { chrono::Utc::now().to_rfc3339() } else { timestamp };
    save_document_at(&conn, &folder, "activity_log", &content, &ts)
}

#[tauri::command]
pub async fn get_report_cmd(db: tauri::State<'_, Db>, id: i64) -> Result<ReportEnvelope, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let (doc_type, content, generated_at): (String, String, String) = conn.query_row(
        "SELECT doc_type, content, generated_at FROM story_documents WHERE id = ?1",
        params![id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))
    ).map_err(|_| "Report not found.".to_string())?;

    let label = label_for_doc_type(&conn, &doc_type);

    let format = if content.starts_with('{') || content.starts_with('[') {
        if serde_json::from_str::<serde_json::Value>(&content).is_ok() { "json" } else { "markdown" }
    } else {
        "markdown"
    };

    Ok(ReportEnvelope { id, doc_type, label, format: format.to_string(), content, generated_at })
}

// ── Delete a report snapshot ────────────────────────

#[tauri::command]
pub async fn delete_report_cmd(db: tauri::State<'_, Db>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let affected = conn
        .execute("DELETE FROM story_documents WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    if affected == 0 {
        return Err("Report not found.".to_string());
    }
    Ok(())
}

// ── Saved reports list (flat, by platform) ───────────────────────────────

#[derive(serde::Serialize, Clone, Debug)]
pub struct SidebarReport {
    pub id:           i64,
    pub doc_type:     String,
    pub label:        String,
    pub generated_at: String,
}

/// Returns current saved reports for a story, filtered by platform.
#[tauri::command]
pub async fn get_sidebar_reports(db: tauri::State<'_, Db>, folder: String, platform: String) -> Result<Vec<SidebarReport>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    // Get report types for this platform
    let mut type_stmt = conn.prepare(
        "SELECT id, label, description FROM report_types ORDER BY rowid"
    ).map_err(|e| e.to_string())?;
    let all_types: Vec<(String, String, String)> = type_stmt.query_map([], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
    }).map_err(|e| e.to_string())?
      .filter_map(|r| r.ok())
      .collect();

    // Get platforms for each type
    let mut plat_stmt = conn.prepare(
        "SELECT id, platforms FROM report_types"
    ).map_err(|e| e.to_string())?;
    let plat_map: std::collections::HashMap<String, Vec<String>> = plat_stmt.query_map([], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    }).map_err(|e| e.to_string())?
      .filter_map(|r| r.ok())
      .map(|(id, platforms)| {
          let plats: Vec<String> = platforms.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
          (id, plats)
      })
      .collect();

    // Get all saved current documents for this folder
    let docs = list_documents(&conn, &folder);

    // Newest current doc per type (list_documents is already newest-first)
    let mut doc_by_type: std::collections::HashMap<String, (i64, String)> = std::collections::HashMap::new();
    for doc in &docs {
        doc_by_type.entry(doc.doc_type.clone()).or_insert((doc.id, doc.generated_at.clone()));
    }

    let mut reports: Vec<SidebarReport> = Vec::new();
    for (id, label, _description) in all_types {
        let included = if platform == "saved" {
            doc_by_type.contains_key(&id)
        } else {
            plat_map.get(&id).map(|p| p.contains(&platform)).unwrap_or(false)
        };
        if !included {
            continue;
        }
        if let Some((doc_id, generated_at)) = doc_by_type.remove(&id) {
            reports.push(SidebarReport { id: doc_id, doc_type: id, label, generated_at });
        }
    }

    Ok(reports)
}

// ── Archived reports (Settings tab) ─────────────────────────────────────────

#[derive(serde::Serialize, Clone, Debug)]
pub struct ArchivedReportRow {
    pub id:            i64,
    pub doc_type:      String,
    pub label:         String,
    pub generated_at:  String,
    pub archived_at:   String,
    pub archive_reason: String,
}

pub fn list_archived_reports(conn: &Connection, story_folder: &str) -> Vec<ArchivedReportRow> {
    let mut stmt = match conn.prepare(
        "SELECT id, doc_type, generated_at, archived_at, archive_reason
         FROM story_documents
         WHERE story_folder = ?1 AND status = 'archived'
         ORDER BY archived_at DESC, id DESC",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let raw: Vec<(i64, String, String, String, String)> = stmt
        .query_map(params![story_folder], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get::<_, Option<String>>(3)?.unwrap_or_default(),
                r.get::<_, Option<String>>(4)?.unwrap_or_default(),
            ))
        })
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    raw.into_iter()
        .map(|(id, doc_type, generated_at, archived_at, archive_reason)| {
            let label = label_for_doc_type(conn, &doc_type);
            ArchivedReportRow {
                id,
                doc_type,
                label,
                generated_at,
                archived_at,
                archive_reason,
            }
        })
        .collect()
}

#[tauri::command]
pub async fn get_archived_reports(
    db: tauri::State<'_, Db>,
    folder: String,
) -> Result<Vec<ArchivedReportRow>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    Ok(list_archived_reports(&conn, &folder))
}

// ── Story artifact state (Settings → Story Data) ─────────────────────────────

#[derive(serde::Serialize, Clone, Debug)]
pub struct ChapterFingerprintRow {
    pub file:          String,
    pub title:         String,
    pub word_count:    i64,
    pub source_hash:   String,
    pub pov:           String,
    pub tense:         String,
    pub dialogue_pct:  i64,
    pub updated_at:    String,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct StoryArtifactStateResponse {
    pub manuscript_fingerprint: String,
    pub fingerprint_updated_at: String,
    pub chapter_count:          usize,
    pub chapters:               Vec<ChapterFingerprintRow>,
    pub artifacts:              Vec<(String, String)>, // (artifact_type, status)
}

#[tauri::command]
pub async fn get_story_artifact_state(
    db: tauri::State<'_, Db>,
    folder: String,
) -> Result<StoryArtifactStateResponse, String> {
    let folder_path = std::path::PathBuf::from(&folder);
    let current_fp =
        crate::analysis::chapters::compute_manuscript_fingerprint(&folder_path);

    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let _ = sync_manuscript_state(&conn, &folder, &current_fp)?;

    let (manuscript_fingerprint, fingerprint_updated_at) = conn
        .query_row(
            "SELECT manuscript_fingerprint, updated_at FROM story_manuscript_state WHERE story_folder = ?1",
            params![folder],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .unwrap_or((current_fp.clone(), String::new()));

    let chapters: Vec<ChapterFingerprintRow> = {
        let mut ch_stmt = conn
            .prepare(
                "SELECT file, title, word_count, source_hash, pov, tense, dialogue_pct, updated_at
                 FROM chapter_fingerprints WHERE story_folder = ?1 ORDER BY file COLLATE NOCASE",
            )
            .map_err(|e| e.to_string())?;
        let rows = ch_stmt
            .query_map(params![folder], |r| {
                Ok(ChapterFingerprintRow {
                    file:         r.get(0)?,
                    title:        r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    word_count:   r.get(2)?,
                    source_hash:  r.get(3)?,
                    pov:          r.get(4)?,
                    tense:        r.get(5)?,
                    dialogue_pct: r.get(6)?,
                    updated_at:   r.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };

    let artifact_types = [
        "fingerprints",
        "genre_data",
        "genre_ranking",
        "categories",
        "kdp_keywords",
        "mi_search_terms",
        "discovery_keywords",
        "keyword_search",
        "bisac",
        "zeigarnik",
    ];
    let artifacts: Vec<(String, String)> = artifact_types
        .iter()
        .map(|t| (t.to_string(), artifact_status(&conn, &folder, t, &manuscript_fingerprint).as_str().to_string()))
        .collect();

    Ok(StoryArtifactStateResponse {
        manuscript_fingerprint,
        fingerprint_updated_at,
        chapter_count: chapters.len(),
        chapters,
        artifacts,
    })
}

#[tauri::command]
pub async fn refresh_chapter_fingerprints(
    app: tauri::AppHandle,
    folder: String,
) -> Result<String, String> {
    let request = crate::analysis::FolderRequest {
        folder,
        provider: String::new(),
        api_key: String::new(),
        model: String::new(),
        genre_model: String::new(),
        canopy_api_key: String::new(),
    };
    let result = crate::analysis::chapters::generate_summaries(app, request).await;
    if result.success {
        Ok(result.report)
    } else {
        Err(result.error)
    }
}

#[tauri::command]
pub async fn clear_chapter_fingerprints(
    db: tauri::State<'_, Db>,
    folder: String,
) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    delete_chapter_summaries(&conn, &folder)?;
    mark_artifacts_stale(&conn, &folder)?;
    Ok(())
}

// ── BISAC classifications ──────────────────────────────────────────────

#[derive(serde::Serialize, Clone, Debug)]
pub struct BisacCodeRow {
    pub code:    String,
    pub heading: String,
}

pub fn master_bisac_list(conn: &Connection) -> Vec<BisacCodeRow> {
    let mut stmt = match conn.prepare("SELECT code, heading FROM bisac_codes ORDER BY code")
        { Ok(s) => s, Err(_) => return Vec::new() };
    stmt.query_map([], |r| Ok(BisacCodeRow { code: r.get(0)?, heading: r.get(1)? }))
        .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
        .unwrap_or_default()
}

/// Replace all stored BISAC classifications for a story+format — latest call
/// wins, same "supersede, don't accumulate" model as genre rankings. `format`
/// is "ebook" or "print", scored and stored independently since a print-only
/// distribution can legitimately warrant different codes than the ebook.
pub fn replace_bisac_classifications(
    conn: &Connection, story_folder: &str, format: &str, rows: &[(String, String, u8, String)],
    // (code, heading, confidence, reason)
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute("DELETE FROM bisac_classifications WHERE story_folder = ?1 AND format = ?2", params![story_folder, format])
        .map_err(|e| e.to_string())?;
    for (code, heading, confidence, reason) in rows {
        conn.execute(
            "INSERT INTO bisac_classifications (story_folder, code, heading, confidence, reason, generated_at, format)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![story_folder, code, heading, confidence, reason, now, format],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn has_bisac_classifications(conn: &Connection, story_folder: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM bisac_classifications WHERE story_folder = ?1 LIMIT 1",
        params![story_folder], |_| Ok(())
    ).is_ok()
}

// ── Top-level KDP categories (derived from catalog) ─────────────────────

/// Extract the distinct top-level segments from all kdp_categories paths for
/// Look up the Amazon node ID for a category path. Returns None if the path
/// isn't in the catalog or has no node ID (manually-added paths without WinningCat data).
pub fn node_id_for_path(conn: &Connection, path: &str, store: &str) -> Option<String> {
    conn.query_row(
        "SELECT amazon_node_id FROM kdp_categories WHERE path = ?1 AND store = ?2 AND amazon_node_id IS NOT NULL AND amazon_node_id != ''",
        params![path, store], |r| r.get::<_, String>(0)
    ).ok()
}

// ── Database inspector (Settings → Database tab) ─────────────────────────────

pub fn database_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("manuscript-intel.db"))
}

fn list_user_table_names(conn: &Connection) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT name FROM sqlite_master
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
             ORDER BY name COLLATE NOCASE",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn table_row_count(conn: &Connection, table: &str) -> Result<i64, String> {
    let sql = format!("SELECT COUNT(*) FROM \"{}\"", table.replace('"', "\"\""));
    conn.query_row(&sql, [], |r| r.get(0))
        .map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
pub struct DbColumnInfo {
    pub name:      String,
    pub type_name: String,
    pub notnull:   bool,
    pub pk:        bool,
}

#[derive(serde::Serialize)]
pub struct DbTableInfo {
    pub name:      String,
    pub row_count: i64,
    pub columns:   Vec<DbColumnInfo>,
}

#[derive(serde::Serialize)]
pub struct DbInspectOverview {
    pub path:            String,
    pub file_size_bytes: u64,
    pub tables:          Vec<DbTableInfo>,
}

#[tauri::command]
pub async fn inspect_database_overview(
    app: AppHandle,
    db: tauri::State<'_, Db>,
) -> Result<DbInspectOverview, String> {
    let path = database_path(&app)?;
    let file_size_bytes = std::fs::metadata(&path)
        .map(|m| m.len())
        .unwrap_or(0);

    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let table_names = list_user_table_names(&conn)?;

    let mut tables = Vec::new();
    for name in table_names {
        let row_count = table_row_count(&conn, &name)?;
        let mut col_stmt = conn
            .prepare(&format!("PRAGMA table_info(\"{}\")", name.replace('"', "\"\"")))
            .map_err(|e| e.to_string())?;
        let columns = col_stmt
            .query_map([], |r| {
                Ok(DbColumnInfo {
                    name:      r.get::<_, String>(1)?,
                    type_name: r.get::<_, String>(2)?,
                    notnull:   r.get::<_, i64>(3)? != 0,
                    pk:        r.get::<_, i64>(5)? != 0,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        tables.push(DbTableInfo { name, row_count, columns });
    }

    Ok(DbInspectOverview {
        path: path.to_string_lossy().into_owned(),
        file_size_bytes,
        tables,
    })
}

#[derive(serde::Deserialize)]
pub struct DbTablePreviewRequest {
    pub table:  String,
    pub offset: Option<i64>,
    pub limit:  Option<i64>,
}

#[derive(serde::Serialize)]
pub struct DbTablePreview {
    pub table:      String,
    pub columns:    Vec<String>,
    pub rows:       Vec<Vec<String>>,
    pub total_rows: i64,
    pub offset:     i64,
    pub limit:      i64,
}

fn is_sensitive_setting_key(key: &str) -> bool {
    matches!(
        key,
        "api_key"
            | "tokenmix_api_key"
            | "canopy_api_key"
            | "dataforseo_password"
    )
}

fn truncate_cell(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let truncated: String = value.chars().take(max).collect();
    format!("{truncated}…")
}

#[tauri::command]
pub async fn inspect_database_table(
    db: tauri::State<'_, Db>,
    request: DbTablePreviewRequest,
) -> Result<DbTablePreview, String> {
    let offset = request.offset.unwrap_or(0).max(0);
    let limit = request.limit.unwrap_or(50).clamp(1, 200);

    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let allowed = list_user_table_names(&conn)?;
    if !allowed.iter().any(|t| t == &request.table) {
        return Err(format!("Unknown table: {}", request.table));
    }

    let table = &request.table;
    let total_rows = table_row_count(&conn, table)?;

    let mut col_stmt = conn
        .prepare(&format!("PRAGMA table_info(\"{}\")", table.replace('"', "\"\"")))
        .map_err(|e| e.to_string())?;
    let mut columns: Vec<String> = vec!["rowid".to_string()];
    columns.extend(
        col_stmt
            .query_map([], |r| r.get::<_, String>(1))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?,
    );

    if columns.len() <= 1 {
        return Ok(DbTablePreview {
            table: table.clone(),
            columns,
            rows: Vec::new(),
            total_rows,
            offset,
            limit,
        });
    }

    let data_col_list = columns[1..]
        .iter()
        .map(|c| format!("\"{}\"", c.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT rowid, {data_col_list} FROM \"{}\" LIMIT ?1 OFFSET ?2",
        table.replace('"', "\"\"")
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let raw_rows = stmt
        .query_map(params![limit, offset], |r| {
            let mut row = Vec::with_capacity(columns.len());
            for i in 0..columns.len() {
                let val: String = match r.get::<_, rusqlite::types::Value>(i) {
                    Ok(rusqlite::types::Value::Null) => String::new(),
                    Ok(rusqlite::types::Value::Integer(n)) => n.to_string(),
                    Ok(rusqlite::types::Value::Real(f)) => f.to_string(),
                    Ok(rusqlite::types::Value::Text(s)) => s,
                    Ok(rusqlite::types::Value::Blob(_)) => "[blob]".to_string(),
                    Err(_) => String::new(),
                };
                row.push(val);
            }
            Ok(row)
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let key_col = columns.iter().position(|c| c == "key");
    let value_col = columns.iter().position(|c| c == "value");

    let rows = raw_rows
        .into_iter()
        .map(|mut row| {
            if table == "app_settings" {
                if let (Some(ki), Some(vi)) = (key_col, value_col) {
                    if row.len() > vi && is_sensitive_setting_key(&row[ki]) && !row[vi].is_empty() {
                        row[vi] = "••••••••".to_string();
                    }
                }
            }
            row.into_iter()
                .map(|cell| truncate_cell(&cell, 280))
                .collect()
        })
        .collect();

    Ok(DbTablePreview {
        table: table.clone(),
        columns,
        rows,
        total_rows,
        offset,
        limit,
    })
}

#[derive(serde::Deserialize)]
pub struct DbDeleteRowRequest {
    pub table: String,
    pub rowid: i64,
}

#[derive(serde::Deserialize)]
pub struct DbUpdateRowRequest {
    pub table:  String,
    pub rowid:    i64,
    pub values:   std::collections::HashMap<String, String>,
}

fn table_columns(conn: &Connection, table: &str) -> Result<Vec<String>, String> {
    let mut col_stmt = conn
        .prepare(&format!("PRAGMA table_info(\"{}\")", table.replace('"', "\"\"")))
        .map_err(|e| e.to_string())?;
    let cols = col_stmt
        .query_map([], |r| r.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(cols)
}

#[tauri::command]
pub async fn delete_database_row_cmd(
    db: tauri::State<'_, Db>,
    request: DbDeleteRowRequest,
) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let allowed = list_user_table_names(&conn)?;
    if !allowed.iter().any(|t| t == &request.table) {
        return Err(format!("Unknown table: {}", request.table));
    }
    let sql = format!(
        "DELETE FROM \"{}\" WHERE rowid = ?1",
        request.table.replace('"', "\"\"")
    );
    let affected = conn
        .execute(&sql, params![request.rowid])
        .map_err(|e| e.to_string())?;
    if affected == 0 {
        return Err("Row not found.".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn update_database_row_cmd(
    db: tauri::State<'_, Db>,
    request: DbUpdateRowRequest,
) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let allowed = list_user_table_names(&conn)?;
    if !allowed.iter().any(|t| t == &request.table) {
        return Err(format!("Unknown table: {}", request.table));
    }

    let table = &request.table;
    let columns = table_columns(&conn, table)?;
    if columns.is_empty() {
        return Err("Table has no columns.".to_string());
    }

    if table == "app_settings" {
        if let Some(key) = request.values.get("key") {
            if is_sensitive_setting_key(key) {
                return Err("Cannot edit sensitive settings keys via the database inspector.".to_string());
            }
        }
        if request.values.contains_key("value") {
            let current_key: String = conn
                .query_row(
                    &format!(
                        "SELECT \"key\" FROM \"{}\" WHERE rowid = ?1",
                        table.replace('"', "\"\"")
                    ),
                    params![request.rowid],
                    |r| r.get(0),
                )
                .map_err(|_| "Row not found.".to_string())?;
            if is_sensitive_setting_key(&current_key) {
                return Err("Cannot edit sensitive settings values via the database inspector.".to_string());
            }
        }
    }

    let mut sets: Vec<String> = Vec::new();
    let mut bind_values: Vec<String> = Vec::new();
    for (col, val) in &request.values {
        if col == "rowid" || !columns.iter().any(|c| c == col) {
            continue;
        }
        sets.push(format!("\"{}\" = ?", col.replace('"', "\"\"")));
        bind_values.push(val.clone());
    }
    if sets.is_empty() {
        return Err("No valid columns to update.".to_string());
    }

    let sql = format!(
        "UPDATE \"{}\" SET {} WHERE rowid = ?",
        table.replace('"', "\"\""),
        sets.join(", ")
    );
    let affected = match bind_values.len() {
        1 => conn.execute(&sql, params![bind_values[0], request.rowid]),
        2 => conn.execute(&sql, params![bind_values[0], bind_values[1], request.rowid]),
        3 => conn.execute(&sql, params![bind_values[0], bind_values[1], bind_values[2], request.rowid]),
        4 => conn.execute(
            &sql,
            params![
                bind_values[0],
                bind_values[1],
                bind_values[2],
                bind_values[3],
                request.rowid
            ],
        ),
        5 => conn.execute(
            &sql,
            params![
                bind_values[0],
                bind_values[1],
                bind_values[2],
                bind_values[3],
                bind_values[4],
                request.rowid
            ],
        ),
        _ => {
            return Err("Too many columns to update at once (max 5).".to_string());
        }
    }
    .map_err(|e| e.to_string())?;
    if affected == 0 {
        return Err("Row not found or update failed.".to_string());
    }
    Ok(())
}

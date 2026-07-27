// campaigns.rs — Ad campaign tracking (Marketing mode)

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use crate::db;

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AdPlatformAccount {
    pub id: i64,
    pub platform: String,
    pub account_id: String,
    pub pixel_id: String,
    pub tracking_notes: String,
    pub payment_notes: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AdLandingPage {
    pub id: i64,
    pub story_folder: String,
    pub name: String,
    pub url: String,
    pub conversion_rate: Option<f64>,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AdCampaign {
    pub id: i64,
    pub story_folder: String,
    pub name: String,
    pub platform: String,
    pub platform_account_id: Option<i64>,
    pub objective: String,
    pub status: String,
    pub budget: Option<f64>,
    pub budget_period: String,
    pub start_date: String,
    pub end_date: String,
    pub target_audience: String,
    pub landing_page_id: Option<i64>,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub total_spend: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AdCreative {
    pub id: i64,
    pub campaign_id: i64,
    pub name: String,
    pub creative_type: String,
    pub version: String,
    pub platform_format: String,
    pub status: String,
    pub asset_path: String,
    pub body_text: String,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AdPerformanceSnapshot {
    pub id: i64,
    pub campaign_id: i64,
    pub creative_id: Option<i64>,
    pub snapshot_date: String,
    pub impressions: i64,
    pub clicks: i64,
    pub conversions: i64,
    pub ctr: f64,
    pub cpc: f64,
    pub cpa: f64,
    pub spend: f64,
    pub notes: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AdSpendEntry {
    pub id: i64,
    pub campaign_id: i64,
    pub platform: String,
    pub amount: f64,
    pub spent_at: String,
    pub notes: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AdAudienceNote {
    pub id: i64,
    pub campaign_id: i64,
    pub label: String,
    pub demographics: String,
    pub interests: String,
    pub lookalike_notes: String,
    pub outcome: String,
    pub notes: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AdCampaignDetail {
    pub campaign: AdCampaign,
    pub creatives: Vec<AdCreative>,
    pub snapshots: Vec<AdPerformanceSnapshot>,
    pub spend_entries: Vec<AdSpendEntry>,
    pub audience_notes: Vec<AdAudienceNote>,
    pub landing_page: Option<AdLandingPage>,
}

#[derive(Serialize)]
pub struct CampaignResult {
    pub success: bool,
    pub error: String,
}

#[derive(Serialize)]
pub struct CampaignListResult {
    pub success: bool,
    pub campaigns: Vec<AdCampaign>,
    pub error: String,
}

#[derive(Serialize)]
pub struct CampaignDetailResult {
    pub success: bool,
    pub detail: Option<AdCampaignDetail>,
    pub error: String,
}

#[derive(Serialize)]
pub struct PlatformAccountsResult {
    pub success: bool,
    pub accounts: Vec<AdPlatformAccount>,
    pub error: String,
}

#[derive(Serialize)]
pub struct LandingPagesResult {
    pub success: bool,
    pub pages: Vec<AdLandingPage>,
    pub error: String,
}

#[derive(Serialize)]
pub struct CreativesResult {
    pub success: bool,
    pub creatives: Vec<AdCreative>,
    pub error: String,
}

#[derive(Serialize)]
pub struct SnapshotsResult {
    pub success: bool,
    pub snapshots: Vec<AdPerformanceSnapshot>,
    pub error: String,
}

#[derive(Serialize)]
pub struct SpendEntriesResult {
    pub success: bool,
    pub entries: Vec<AdSpendEntry>,
    pub error: String,
}

#[derive(Serialize)]
pub struct AudienceNotesResult {
    pub success: bool,
    pub notes: Vec<AdAudienceNote>,
    pub error: String,
}

#[derive(Serialize)]
pub struct IdResult {
    pub success: bool,
    pub id: i64,
    pub error: String,
}

// ── Request types ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateCampaignRequest {
    pub story_folder: String,
    pub name: String,
    #[serde(default)]
    pub platform: String,
    #[serde(default)]
    pub objective: String,
}

#[derive(Deserialize)]
pub struct UpdateCampaignRequest {
    pub id: i64,
    pub name: String,
    pub platform: String,
    pub platform_account_id: Option<i64>,
    pub objective: String,
    pub status: String,
    pub budget: Option<f64>,
    pub budget_period: String,
    pub start_date: String,
    pub end_date: String,
    pub target_audience: String,
    pub landing_page_id: Option<i64>,
    pub notes: String,
}

#[derive(Deserialize)]
pub struct PlatformAccountInput {
    pub platform: String,
    #[serde(default)]
    pub account_id: String,
    #[serde(default)]
    pub pixel_id: String,
    #[serde(default)]
    pub tracking_notes: String,
    #[serde(default)]
    pub payment_notes: String,
}

#[derive(Deserialize)]
pub struct UpdatePlatformAccountRequest {
    pub id: i64,
    pub platform: String,
    pub account_id: String,
    pub pixel_id: String,
    pub tracking_notes: String,
    pub payment_notes: String,
}

#[derive(Deserialize)]
pub struct LandingPageInput {
    pub story_folder: String,
    pub name: String,
    #[serde(default)]
    pub url: String,
    pub conversion_rate: Option<f64>,
    #[serde(default)]
    pub notes: String,
}

#[derive(Deserialize)]
pub struct UpdateLandingPageRequest {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub conversion_rate: Option<f64>,
    pub notes: String,
}

#[derive(Deserialize)]
pub struct CreativeInput {
    pub campaign_id: i64,
    pub name: String,
    #[serde(default)]
    pub creative_type: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub platform_format: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub asset_path: String,
    #[serde(default)]
    pub body_text: String,
    #[serde(default)]
    pub notes: String,
}

#[derive(Deserialize)]
pub struct UpdateCreativeRequest {
    pub id: i64,
    pub name: String,
    pub creative_type: String,
    pub version: String,
    pub platform_format: String,
    pub status: String,
    pub asset_path: String,
    pub body_text: String,
    pub notes: String,
}

#[derive(Deserialize)]
pub struct SnapshotInput {
    pub campaign_id: i64,
    pub creative_id: Option<i64>,
    pub snapshot_date: String,
    #[serde(default)]
    pub impressions: i64,
    #[serde(default)]
    pub clicks: i64,
    #[serde(default)]
    pub conversions: i64,
    #[serde(default)]
    pub ctr: f64,
    #[serde(default)]
    pub cpc: f64,
    #[serde(default)]
    pub cpa: f64,
    #[serde(default)]
    pub spend: f64,
    #[serde(default)]
    pub notes: String,
}

#[derive(Deserialize)]
pub struct SpendInput {
    pub campaign_id: i64,
    #[serde(default)]
    pub platform: String,
    pub amount: f64,
    pub spent_at: String,
    #[serde(default)]
    pub notes: String,
}

#[derive(Deserialize)]
pub struct AudienceNoteInput {
    pub campaign_id: i64,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub demographics: String,
    #[serde(default)]
    pub interests: String,
    #[serde(default)]
    pub lookalike_notes: String,
    #[serde(default)]
    pub outcome: String,
    #[serde(default)]
    pub notes: String,
}

#[derive(Deserialize)]
pub struct UpdateAudienceNoteRequest {
    pub id: i64,
    pub label: String,
    pub demographics: String,
    pub interests: String,
    pub lookalike_notes: String,
    pub outcome: String,
    pub notes: String,
}

// ── Row loaders ───────────────────────────────────────────────────────────────

fn load_campaign_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<AdCampaign> {
    Ok(AdCampaign {
        id: r.get(0)?,
        story_folder: r.get(1)?,
        name: r.get(2)?,
        platform: r.get(3)?,
        platform_account_id: r.get(4)?,
        objective: r.get(5)?,
        status: r.get(6)?,
        budget: r.get(7)?,
        budget_period: r.get(8)?,
        start_date: r.get(9)?,
        end_date: r.get(10)?,
        target_audience: r.get(11)?,
        landing_page_id: r.get(12)?,
        notes: r.get(13)?,
        created_at: r.get(14)?,
        updated_at: r.get(15)?,
        total_spend: 0.0,
    })
}

fn campaign_spend_total(conn: &rusqlite::Connection, campaign_id: i64) -> f64 {
    conn.query_row(
        "SELECT COALESCE(SUM(amount), 0) FROM ad_spend_entries WHERE campaign_id = ?1",
        rusqlite::params![campaign_id],
        |r| r.get(0),
    ).unwrap_or(0.0)
}

fn load_landing_page(conn: &rusqlite::Connection, id: i64) -> Option<AdLandingPage> {
    conn.query_row(
        "SELECT id, story_folder, name, url, conversion_rate, notes, created_at, updated_at FROM ad_landing_pages WHERE id = ?1",
        rusqlite::params![id],
        |r| Ok(AdLandingPage {
            id: r.get(0)?,
            story_folder: r.get(1)?,
            name: r.get(2)?,
            url: r.get(3)?,
            conversion_rate: r.get(4)?,
            notes: r.get(5)?,
            created_at: r.get(6)?,
            updated_at: r.get(7)?,
        }),
    ).ok()
}

// ── Campaign commands ─────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_campaigns(app: AppHandle, story_folder: String) -> CampaignListResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    let mut stmt = match conn.prepare(
        "SELECT id, story_folder, name, platform, platform_account_id, objective, status,
                budget, budget_period, start_date, end_date, target_audience, landing_page_id,
                notes, created_at, updated_at
         FROM ad_campaigns WHERE story_folder = ?1 ORDER BY updated_at DESC"
    ) {
        Ok(s) => s,
        Err(e) => return CampaignListResult { success: false, campaigns: Vec::new(), error: e.to_string() },
    };
    let mut campaigns: Vec<AdCampaign> = stmt.query_map(rusqlite::params![story_folder], load_campaign_row)
        .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
        .unwrap_or_default();
    for c in &mut campaigns {
        c.total_spend = campaign_spend_total(&conn, c.id);
    }
    CampaignListResult { success: true, campaigns, error: String::new() }
}

#[tauri::command]
pub async fn create_campaign(app: AppHandle, request: CreateCampaignRequest) -> IdResult {
    let name = request.name.trim();
    if name.is_empty() {
        return IdResult { success: false, id: 0, error: "Campaign name is required.".to_string() };
    }
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    let ts = now();
    if let Err(e) = conn.execute(
        "INSERT INTO ad_campaigns (story_folder, name, platform, objective, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![request.story_folder, name, request.platform, request.objective, ts, ts],
    ) {
        return IdResult { success: false, id: 0, error: e.to_string() };
    }
    IdResult { success: true, id: conn.last_insert_rowid(), error: String::new() }
}

#[tauri::command]
pub async fn update_campaign(app: AppHandle, request: UpdateCampaignRequest) -> CampaignResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    if let Err(e) = conn.execute(
        "UPDATE ad_campaigns SET name=?1, platform=?2, platform_account_id=?3, objective=?4,
         status=?5, budget=?6, budget_period=?7, start_date=?8, end_date=?9, target_audience=?10,
         landing_page_id=?11, notes=?12, updated_at=?13 WHERE id=?14",
        rusqlite::params![
            request.name.trim(), request.platform, request.platform_account_id, request.objective,
            request.status, request.budget, request.budget_period, request.start_date, request.end_date,
            request.target_audience, request.landing_page_id, request.notes, now(), request.id,
        ],
    ) {
        return CampaignResult { success: false, error: e.to_string() };
    }
    CampaignResult { success: true, error: String::new() }
}

#[tauri::command]
pub async fn delete_campaign(app: AppHandle, id: i64) -> CampaignResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    if let Err(e) = conn.execute("DELETE FROM ad_campaigns WHERE id = ?1", rusqlite::params![id]) {
        return CampaignResult { success: false, error: e.to_string() };
    }
    CampaignResult { success: true, error: String::new() }
}

#[tauri::command]
pub async fn get_campaign_detail(app: AppHandle, id: i64) -> CampaignDetailResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    let mut campaign: AdCampaign = match conn.query_row(
        "SELECT id, story_folder, name, platform, platform_account_id, objective, status,
                budget, budget_period, start_date, end_date, target_audience, landing_page_id,
                notes, created_at, updated_at FROM ad_campaigns WHERE id = ?1",
        rusqlite::params![id],
        load_campaign_row,
    ) {
        Ok(c) => c,
        Err(e) => return CampaignDetailResult { success: false, detail: None, error: e.to_string() },
    };
    campaign.total_spend = campaign_spend_total(&conn, id);

    let creatives = load_creatives_inner(&conn, id);
    let snapshots = load_snapshots_inner(&conn, id);
    let spend_entries = load_spend_inner(&conn, id);
    let audience_notes = load_audience_inner(&conn, id);
    let landing_page = campaign.landing_page_id.and_then(|lp_id| load_landing_page(&conn, lp_id));

    CampaignDetailResult {
        success: true,
        detail: Some(AdCampaignDetail { campaign, creatives, snapshots, spend_entries, audience_notes, landing_page }),
        error: String::new(),
    }
}

// ── Creatives ─────────────────────────────────────────────────────────────────

fn load_creatives_inner(conn: &rusqlite::Connection, campaign_id: i64) -> Vec<AdCreative> {
    let mut stmt = match conn.prepare(
        "SELECT id, campaign_id, name, creative_type, version, platform_format, status,
                asset_path, body_text, notes, created_at, updated_at
         FROM ad_creatives WHERE campaign_id = ?1 ORDER BY created_at"
    ) { Ok(s) => s, Err(_) => return Vec::new() };
    stmt.query_map(rusqlite::params![campaign_id], |r| Ok(AdCreative {
        id: r.get(0)?, campaign_id: r.get(1)?, name: r.get(2)?, creative_type: r.get(3)?,
        version: r.get(4)?, platform_format: r.get(5)?, status: r.get(6)?, asset_path: r.get(7)?,
        body_text: r.get(8)?, notes: r.get(9)?, created_at: r.get(10)?, updated_at: r.get(11)?,
    })).and_then(|rows| rows.collect()).unwrap_or_default()
}

#[tauri::command]
pub async fn list_creatives(app: AppHandle, campaign_id: i64) -> CreativesResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    CreativesResult { success: true, creatives: load_creatives_inner(&conn, campaign_id), error: String::new() }
}

#[tauri::command]
pub async fn create_creative(app: AppHandle, request: CreativeInput) -> IdResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    let ts = now();
    if let Err(e) = conn.execute(
        "INSERT INTO ad_creatives (campaign_id, name, creative_type, version, platform_format, status, asset_path, body_text, notes, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        rusqlite::params![request.campaign_id, request.name, request.creative_type, request.version,
            request.platform_format, request.status, request.asset_path, request.body_text, request.notes, ts, ts],
    ) {
        return IdResult { success: false, id: 0, error: e.to_string() };
    }
    IdResult { success: true, id: conn.last_insert_rowid(), error: String::new() }
}

#[tauri::command]
pub async fn update_creative(app: AppHandle, request: UpdateCreativeRequest) -> CampaignResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    if let Err(e) = conn.execute(
        "UPDATE ad_creatives SET name=?1, creative_type=?2, version=?3, platform_format=?4, status=?5,
         asset_path=?6, body_text=?7, notes=?8, updated_at=?9 WHERE id=?10",
        rusqlite::params![request.name, request.creative_type, request.version, request.platform_format,
            request.status, request.asset_path, request.body_text, request.notes, now(), request.id],
    ) {
        return CampaignResult { success: false, error: e.to_string() };
    }
    CampaignResult { success: true, error: String::new() }
}

#[tauri::command]
pub async fn delete_creative(app: AppHandle, id: i64) -> CampaignResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    if let Err(e) = conn.execute("DELETE FROM ad_creatives WHERE id = ?1", rusqlite::params![id]) {
        return CampaignResult { success: false, error: e.to_string() };
    }
    CampaignResult { success: true, error: String::new() }
}

// ── Performance snapshots ─────────────────────────────────────────────────────

fn load_snapshots_inner(conn: &rusqlite::Connection, campaign_id: i64) -> Vec<AdPerformanceSnapshot> {
    let mut stmt = match conn.prepare(
        "SELECT id, campaign_id, creative_id, snapshot_date, impressions, clicks, conversions,
                ctr, cpc, cpa, spend, notes, created_at
         FROM ad_performance_snapshots WHERE campaign_id = ?1 ORDER BY snapshot_date DESC"
    ) { Ok(s) => s, Err(_) => return Vec::new() };
    stmt.query_map(rusqlite::params![campaign_id], |r| Ok(AdPerformanceSnapshot {
        id: r.get(0)?, campaign_id: r.get(1)?, creative_id: r.get(2)?, snapshot_date: r.get(3)?,
        impressions: r.get(4)?, clicks: r.get(5)?, conversions: r.get(6)?, ctr: r.get(7)?,
        cpc: r.get(8)?, cpa: r.get(9)?, spend: r.get(10)?, notes: r.get(11)?, created_at: r.get(12)?,
    })).and_then(|rows| rows.collect()).unwrap_or_default()
}

#[tauri::command]
pub async fn list_performance_snapshots(app: AppHandle, campaign_id: i64) -> SnapshotsResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    SnapshotsResult { success: true, snapshots: load_snapshots_inner(&conn, campaign_id), error: String::new() }
}

#[tauri::command]
pub async fn add_performance_snapshot(app: AppHandle, request: SnapshotInput) -> IdResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    if let Err(e) = conn.execute(
        "INSERT INTO ad_performance_snapshots (campaign_id, creative_id, snapshot_date, impressions, clicks, conversions, ctr, cpc, cpa, spend, notes, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        rusqlite::params![request.campaign_id, request.creative_id, request.snapshot_date,
            request.impressions, request.clicks, request.conversions, request.ctr, request.cpc,
            request.cpa, request.spend, request.notes, now()],
    ) {
        return IdResult { success: false, id: 0, error: e.to_string() };
    }
    IdResult { success: true, id: conn.last_insert_rowid(), error: String::new() }
}

#[tauri::command]
pub async fn delete_performance_snapshot(app: AppHandle, id: i64) -> CampaignResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    if let Err(e) = conn.execute("DELETE FROM ad_performance_snapshots WHERE id = ?1", rusqlite::params![id]) {
        return CampaignResult { success: false, error: e.to_string() };
    }
    CampaignResult { success: true, error: String::new() }
}

// ── Spend entries ───────────────────────────────────────────────────────────

fn load_spend_inner(conn: &rusqlite::Connection, campaign_id: i64) -> Vec<AdSpendEntry> {
    let mut stmt = match conn.prepare(
        "SELECT id, campaign_id, platform, amount, spent_at, notes, created_at
         FROM ad_spend_entries WHERE campaign_id = ?1 ORDER BY spent_at DESC"
    ) { Ok(s) => s, Err(_) => return Vec::new() };
    stmt.query_map(rusqlite::params![campaign_id], |r| Ok(AdSpendEntry {
        id: r.get(0)?, campaign_id: r.get(1)?, platform: r.get(2)?, amount: r.get(3)?,
        spent_at: r.get(4)?, notes: r.get(5)?, created_at: r.get(6)?,
    })).and_then(|rows| rows.collect()).unwrap_or_default()
}

#[tauri::command]
pub async fn list_spend_entries(app: AppHandle, campaign_id: i64) -> SpendEntriesResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    SpendEntriesResult { success: true, entries: load_spend_inner(&conn, campaign_id), error: String::new() }
}

#[tauri::command]
pub async fn add_spend_entry(app: AppHandle, request: SpendInput) -> IdResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    if let Err(e) = conn.execute(
        "INSERT INTO ad_spend_entries (campaign_id, platform, amount, spent_at, notes, created_at)
         VALUES (?1,?2,?3,?4,?5,?6)",
        rusqlite::params![request.campaign_id, request.platform, request.amount, request.spent_at, request.notes, now()],
    ) {
        return IdResult { success: false, id: 0, error: e.to_string() };
    }
    IdResult { success: true, id: conn.last_insert_rowid(), error: String::new() }
}

#[tauri::command]
pub async fn delete_spend_entry(app: AppHandle, id: i64) -> CampaignResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    if let Err(e) = conn.execute("DELETE FROM ad_spend_entries WHERE id = ?1", rusqlite::params![id]) {
        return CampaignResult { success: false, error: e.to_string() };
    }
    CampaignResult { success: true, error: String::new() }
}

// ── Landing pages ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_landing_pages(app: AppHandle, story_folder: String) -> LandingPagesResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    let mut stmt = match conn.prepare(
        "SELECT id, story_folder, name, url, conversion_rate, notes, created_at, updated_at
         FROM ad_landing_pages WHERE story_folder = ?1 ORDER BY name"
    ) { Ok(s) => s, Err(e) => return LandingPagesResult { success: false, pages: Vec::new(), error: e.to_string() } };
    let pages = stmt.query_map(rusqlite::params![story_folder], |r| Ok(AdLandingPage {
        id: r.get(0)?, story_folder: r.get(1)?, name: r.get(2)?, url: r.get(3)?,
        conversion_rate: r.get(4)?, notes: r.get(5)?, created_at: r.get(6)?, updated_at: r.get(7)?,
    })).and_then(|rows| rows.collect()).unwrap_or_default();
    LandingPagesResult { success: true, pages, error: String::new() }
}

#[tauri::command]
pub async fn create_landing_page(app: AppHandle, request: LandingPageInput) -> IdResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    let ts = now();
    if let Err(e) = conn.execute(
        "INSERT INTO ad_landing_pages (story_folder, name, url, conversion_rate, notes, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
        rusqlite::params![request.story_folder, request.name, request.url, request.conversion_rate, request.notes, ts, ts],
    ) {
        return IdResult { success: false, id: 0, error: e.to_string() };
    }
    IdResult { success: true, id: conn.last_insert_rowid(), error: String::new() }
}

#[tauri::command]
pub async fn update_landing_page(app: AppHandle, request: UpdateLandingPageRequest) -> CampaignResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    if let Err(e) = conn.execute(
        "UPDATE ad_landing_pages SET name=?1, url=?2, conversion_rate=?3, notes=?4, updated_at=?5 WHERE id=?6",
        rusqlite::params![request.name, request.url, request.conversion_rate, request.notes, now(), request.id],
    ) {
        return CampaignResult { success: false, error: e.to_string() };
    }
    CampaignResult { success: true, error: String::new() }
}

#[tauri::command]
pub async fn delete_landing_page(app: AppHandle, id: i64) -> CampaignResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    let _ = conn.execute("UPDATE ad_campaigns SET landing_page_id = NULL WHERE landing_page_id = ?1", rusqlite::params![id]);
    if let Err(e) = conn.execute("DELETE FROM ad_landing_pages WHERE id = ?1", rusqlite::params![id]) {
        return CampaignResult { success: false, error: e.to_string() };
    }
    CampaignResult { success: true, error: String::new() }
}

// ── Audience notes ────────────────────────────────────────────────────────────

fn load_audience_inner(conn: &rusqlite::Connection, campaign_id: i64) -> Vec<AdAudienceNote> {
    let mut stmt = match conn.prepare(
        "SELECT id, campaign_id, label, demographics, interests, lookalike_notes, outcome, notes, created_at
         FROM ad_audience_notes WHERE campaign_id = ?1 ORDER BY created_at DESC"
    ) { Ok(s) => s, Err(_) => return Vec::new() };
    stmt.query_map(rusqlite::params![campaign_id], |r| Ok(AdAudienceNote {
        id: r.get(0)?, campaign_id: r.get(1)?, label: r.get(2)?, demographics: r.get(3)?,
        interests: r.get(4)?, lookalike_notes: r.get(5)?, outcome: r.get(6)?, notes: r.get(7)?,
        created_at: r.get(8)?,
    })).and_then(|rows| rows.collect()).unwrap_or_default()
}

#[tauri::command]
pub async fn list_audience_notes(app: AppHandle, campaign_id: i64) -> AudienceNotesResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    AudienceNotesResult { success: true, notes: load_audience_inner(&conn, campaign_id), error: String::new() }
}

#[tauri::command]
pub async fn add_audience_note(app: AppHandle, request: AudienceNoteInput) -> IdResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    if let Err(e) = conn.execute(
        "INSERT INTO ad_audience_notes (campaign_id, label, demographics, interests, lookalike_notes, outcome, notes, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        rusqlite::params![request.campaign_id, request.label, request.demographics, request.interests,
            request.lookalike_notes, request.outcome, request.notes, now()],
    ) {
        return IdResult { success: false, id: 0, error: e.to_string() };
    }
    IdResult { success: true, id: conn.last_insert_rowid(), error: String::new() }
}

#[tauri::command]
pub async fn update_audience_note(app: AppHandle, request: UpdateAudienceNoteRequest) -> CampaignResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    if let Err(e) = conn.execute(
        "UPDATE ad_audience_notes SET label=?1, demographics=?2, interests=?3, lookalike_notes=?4, outcome=?5, notes=?6 WHERE id=?7",
        rusqlite::params![request.label, request.demographics, request.interests, request.lookalike_notes, request.outcome, request.notes, request.id],
    ) {
        return CampaignResult { success: false, error: e.to_string() };
    }
    CampaignResult { success: true, error: String::new() }
}

#[tauri::command]
pub async fn delete_audience_note(app: AppHandle, id: i64) -> CampaignResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    if let Err(e) = conn.execute("DELETE FROM ad_audience_notes WHERE id = ?1", rusqlite::params![id]) {
        return CampaignResult { success: false, error: e.to_string() };
    }
    CampaignResult { success: true, error: String::new() }
}

// ── Platform accounts ─────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_platform_accounts(app: AppHandle) -> PlatformAccountsResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    let mut stmt = match conn.prepare(
        "SELECT id, platform, account_id, pixel_id, tracking_notes, payment_notes, created_at, updated_at
         FROM ad_platform_accounts ORDER BY platform, account_id"
    ) { Ok(s) => s, Err(e) => return PlatformAccountsResult { success: false, accounts: Vec::new(), error: e.to_string() } };
    let accounts = stmt.query_map([], |r| Ok(AdPlatformAccount {
        id: r.get(0)?, platform: r.get(1)?, account_id: r.get(2)?, pixel_id: r.get(3)?,
        tracking_notes: r.get(4)?, payment_notes: r.get(5)?, created_at: r.get(6)?, updated_at: r.get(7)?,
    })).and_then(|rows| rows.collect()).unwrap_or_default();
    PlatformAccountsResult { success: true, accounts, error: String::new() }
}

#[tauri::command]
pub async fn create_platform_account(app: AppHandle, request: PlatformAccountInput) -> IdResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    let ts = now();
    if let Err(e) = conn.execute(
        "INSERT INTO ad_platform_accounts (platform, account_id, pixel_id, tracking_notes, payment_notes, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
        rusqlite::params![request.platform, request.account_id, request.pixel_id, request.tracking_notes, request.payment_notes, ts, ts],
    ) {
        return IdResult { success: false, id: 0, error: e.to_string() };
    }
    IdResult { success: true, id: conn.last_insert_rowid(), error: String::new() }
}

#[tauri::command]
pub async fn update_platform_account(app: AppHandle, request: UpdatePlatformAccountRequest) -> CampaignResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    if let Err(e) = conn.execute(
        "UPDATE ad_platform_accounts SET platform=?1, account_id=?2, pixel_id=?3, tracking_notes=?4, payment_notes=?5, updated_at=?6 WHERE id=?7",
        rusqlite::params![request.platform, request.account_id, request.pixel_id, request.tracking_notes, request.payment_notes, now(), request.id],
    ) {
        return CampaignResult { success: false, error: e.to_string() };
    }
    CampaignResult { success: true, error: String::new() }
}

#[tauri::command]
pub async fn delete_platform_account(app: AppHandle, id: i64) -> CampaignResult {
    let database = app.state::<db::Db>();
    let conn = database.0.lock().unwrap();
    let _ = conn.execute("UPDATE ad_campaigns SET platform_account_id = NULL WHERE platform_account_id = ?1", rusqlite::params![id]);
    if let Err(e) = conn.execute("DELETE FROM ad_platform_accounts WHERE id = ?1", rusqlite::params![id]) {
        return CampaignResult { success: false, error: e.to_string() };
    }
    CampaignResult { success: true, error: String::new() }
}

// Desktop app settings and authentication state

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::db::Db;
use crate::local_ai::DEFAULT_LOCAL_MODEL;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    pub theme: String,
    pub provider: String,
    pub api_key: String,
    pub model_assignments: String,
    pub local_default_model: String,
    pub local_model_assignments: String,
    pub canopy_api_key: String,
    pub dataforseo_login: String,
    pub dataforseo_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub platform: String,
    pub active_story_id: String,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            provider: "local".to_string(),
            api_key: String::new(),
            model_assignments: "{}".to_string(),
            local_default_model: DEFAULT_LOCAL_MODEL.to_string(),
            local_model_assignments: "{}".to_string(),
            canopy_api_key: String::new(),
            dataforseo_login: String::new(),
            dataforseo_password: String::new(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            platform: "kdp".to_string(),
            active_story_id: String::new(),
        }
    }
}


fn load_key_value(conn: &Connection, key: &str) -> String {
    conn.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        params![key],
        |r| r.get::<_, String>(0),
    ).unwrap_or_default()
}

fn normalize_provider(value: &str) -> String {
    if value == "tokenmix" {
        "tokenmix".to_string()
    } else {
        "local".to_string()
    }
}

#[tauri::command]
pub async fn load_ui_settings(db: tauri::State<'_, Db>) -> Result<UiSettings, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let provider_raw = load_key_value(&conn, "provider");
    let provider = if provider_raw.is_empty() {
        "local".to_string()
    } else {
        normalize_provider(&provider_raw)
    };
    let local_default = load_key_value(&conn, "local_default_model");
    Ok(UiSettings {
        theme: load_key_value(&conn, "theme"),
        provider,
        api_key: load_key_value(&conn, "api_key"),
        model_assignments: load_key_value(&conn, "model_assignments"),
        local_default_model: if local_default.is_empty() {
            DEFAULT_LOCAL_MODEL.to_string()
        } else {
            local_default
        },
        local_model_assignments: load_key_value(&conn, "local_model_assignments"),
        canopy_api_key: load_key_value(&conn, "canopy_api_key"),
        dataforseo_login: load_key_value(&conn, "dataforseo_login"),
        dataforseo_password: load_key_value(&conn, "dataforseo_password"),
    })
}

#[tauri::command]
pub async fn load_app_state(db: tauri::State<'_, Db>) -> Result<AppState, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    Ok(AppState {
        platform: load_key_value(&conn, "platform"),
        active_story_id: load_key_value(&conn, "active_story_id"),
    })
}

#[tauri::command]
pub async fn save_ui_settings(
    db: tauri::State<'_, Db>,
    mut settings: UiSettings,
) -> Result<UiSettings, String> {
    settings.provider = normalize_provider(&settings.provider);
    if settings.local_default_model.trim().is_empty() {
        settings.local_default_model = DEFAULT_LOCAL_MODEL.to_string();
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    for (key, value) in [
        ("theme", settings.theme.as_str()),
        ("provider", settings.provider.as_str()),
        ("api_key", settings.api_key.as_str()),
        ("model_assignments", settings.model_assignments.as_str()),
        ("local_default_model", settings.local_default_model.as_str()),
        ("local_model_assignments", settings.local_model_assignments.as_str()),
        ("canopy_api_key", settings.canopy_api_key.as_str()),
        ("dataforseo_login", settings.dataforseo_login.as_str()),
        ("dataforseo_password", settings.dataforseo_password.as_str()),
    ] {
        tx.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)\n             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        ).map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(settings)
}

#[tauri::command]
pub async fn save_app_state(
    db: tauri::State<'_, Db>,
    state: AppState,
) -> Result<AppState, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    for (key, value) in [
        ("platform", state.platform.as_str()),
        ("active_story_id", state.active_story_id.as_str()),
    ] {
        tx.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)\n             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        ).map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(state)
}

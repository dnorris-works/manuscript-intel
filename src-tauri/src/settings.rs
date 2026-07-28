// Desktop app settings and authentication state

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::db::Db;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    pub theme: String,
    pub api_key: String,
    pub model_assignments: String,
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
            api_key: String::new(),
            model_assignments: "{}".to_string(),
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

#[tauri::command]
pub async fn load_ui_settings(db: tauri::State<'_, Db>) -> Result<UiSettings, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut api_key = load_key_value(&conn, "api_key");
    if api_key.is_empty() {
        api_key = load_key_value(&conn, "tokenmix_api_key");
    }
    Ok(UiSettings {
        theme: load_key_value(&conn, "theme"),
        api_key,
        model_assignments: load_key_value(&conn, "model_assignments"),
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
    settings: UiSettings,
) -> Result<UiSettings, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    for (key, value) in [
        ("theme", settings.theme.as_str()),
        ("provider", "tokenmix"),
        ("api_key", settings.api_key.as_str()),
        ("model_assignments", settings.model_assignments.as_str()),
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

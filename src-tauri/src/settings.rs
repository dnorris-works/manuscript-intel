// Desktop app settings and authentication state
#![allow(dead_code)]

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

use crate::db::Db;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Clerk JWT token
    pub clerk_token: Option<String>,
    /// Unique device ID for this desktop installation
    pub device_id: String,
    /// User's email (from Clerk)
    pub user_email: Option<String>,
    /// API base URL
    pub api_url: String,
    /// Last sync timestamp
    pub last_sync: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    pub theme: String,
    pub provider: String,
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
            provider: "tokenmix".to_string(),
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

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            clerk_token: None,
            device_id: Uuid::new_v4().to_string(),
            user_email: None,
            api_url: if cfg!(debug_assertions) {
                "http://localhost:3000".to_string()
            } else {
                "https://api.loremetry.com".to_string()
            },
            last_sync: None,
        }
    }
}

impl AppSettings {
    /// Load settings from disk
    pub fn load(settings_path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        if settings_path.exists() {
            let json = fs::read_to_string(settings_path)?;
            Ok(serde_json::from_str(&json)?)
        } else {
            Ok(Self::default())
        }
    }

    /// Save settings to disk
    pub fn save(&self, settings_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(settings_path, json)?;
        Ok(())
    }

    /// Check if user is authenticated (has Clerk token)
    pub fn is_authenticated(&self) -> bool {
        self.clerk_token.is_some() && !self.clerk_token.as_ref().unwrap().is_empty()
    }

    /// Set Clerk token and email
    pub fn set_auth(&mut self, token: String, email: String) {
        self.clerk_token = Some(token);
        self.user_email = Some(email);
    }

    /// Clear authentication
    pub fn clear_auth(&mut self) {
        self.clerk_token = None;
        self.user_email = None;
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
    Ok(UiSettings {
        theme: load_key_value(&conn, "theme"),
        provider: load_key_value(&conn, "provider"),
        api_key: load_key_value(&conn, "api_key"),
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
        ("provider", settings.provider.as_str()),
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

// Desktop app settings and authentication state
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

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

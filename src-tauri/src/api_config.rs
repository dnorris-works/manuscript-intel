// Configuration for API endpoints and behavior
#![allow(dead_code)]

#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// Base URL for the Web API
    pub base_url: String,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Whether to skip SSL verification (dev only)
    pub insecure: bool,
}

impl Default for ApiConfig {
    fn default() -> Self {
        let base_url = if cfg!(debug_assertions) {
            "http://localhost:3000".to_string()
        } else {
            // In production, use the environment variable or fallback to production URL
            std::env::var("LOREMETRY_API_URL")
                .unwrap_or_else(|_| "https://api.loremetry.com".to_string())
        };

        Self {
            base_url,
            timeout_secs: 30,
            insecure: cfg!(debug_assertions),
        }
    }
}

impl ApiConfig {
    /// Create config from URL string
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            ..Default::default()
        }
    }

    /// Get license check endpoint URL
    pub fn license_check_url(&self) -> String {
        format!("{}/api/license/check", self.base_url)
    }

    /// Get usage report endpoint URL
    pub fn usage_report_url(&self) -> String {
        format!("{}/api/usage/report", self.base_url)
    }

    /// Get usage history endpoint URL
    pub fn usage_history_url(&self) -> String {
        format!("{}/api/usage/history", self.base_url)
    }

    /// Get auth verification endpoint URL
    pub fn auth_verify_url(&self) -> String {
        format!("{}/api/auth/verify", self.base_url)
    }
}

// Type definitions for Web API requests/responses
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────
// License Check
// ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct LicenseCheckRequest {
    // Clerk JWT token sent in Authorization header
    // No body needed - token is in headers
}

#[derive(Debug, Deserialize)]
pub struct LicenseCheckResponse {
    pub valid: bool,
    pub tokens_remaining: i64,
    pub plan: String,
    #[serde(default)]
    pub message: String,
}

// ─────────────────────────────────────────────────────────
// Usage Report
// ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct UsageReportRequest {
    pub tokens_consumed: i64,
    pub analysis_type: String,
    pub device_id: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct UsageReportResponse {
    pub success: bool,
    pub tokens_deducted: i64,
    #[serde(default)]
    pub message: String,
}

// ─────────────────────────────────────────────────────────
// Usage History
// ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct UsageHistoryResponse {
    pub plan: String,
    pub tokens_this_month: i64,
    pub tokens_remaining: i64,
    pub period_start: String,
    pub period_end: String,
    pub recent_usage: Vec<UsageEvent>,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct UsageEvent {
    pub analysis_type: String,
    pub tokens_consumed: i64,
    pub timestamp: String,
    pub status: String,
}

// ─────────────────────────────────────────────────────────
// Auth Verification
// ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AuthVerifyResponse {
    pub valid: bool,
    pub user_id: Option<String>,
}

// ─────────────────────────────────────────────────────────
// Error Response
// ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ApiErrorResponse {
    pub error: String,
    #[serde(default)]
    pub details: serde_json::Value,
}

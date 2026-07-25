// Client for communicating with Loremetry Web API
#![allow(dead_code)]

use reqwest::Client;
use std::sync::Arc;

use crate::api_config::ApiConfig;
use crate::api_types::*;

#[derive(Clone)]
pub struct ApiClient {
    client: Arc<Client>,
    config: ApiConfig,
}

#[derive(Debug)]
pub enum ApiError {
    RequestError(String),
    ResponseError(String),
    AuthenticationError(String),
    QuotaExceeded(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::RequestError(msg) => write!(f, "Request error: {}", msg),
            ApiError::ResponseError(msg) => write!(f, "Response error: {}", msg),
            ApiError::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
            ApiError::QuotaExceeded(msg) => write!(f, "Quota exceeded: {}", msg),
        }
    }
}

impl ApiClient {
    /// Create a new API client with default configuration
    pub fn new() -> Self {
        Self::with_config(ApiConfig::default())
    }

    /// Create a new API client with custom configuration
    pub fn with_config(config: ApiConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .danger_accept_invalid_certs(config.insecure)
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client: Arc::new(client),
            config,
        }
    }

    /// Check license validity and get token quota
    pub async fn check_license(&self, clerk_token: &str) -> Result<LicenseCheckResponse, ApiError> {
        let url = self.config.license_check_url();

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", clerk_token))
            .send()
            .await
            .map_err(|e| ApiError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            
            if status == 401 {
                return Err(ApiError::AuthenticationError(text));
            }
            return Err(ApiError::ResponseError(format!(
                "Status {}: {}",
                status, text
            )));
        }

        response
            .json::<LicenseCheckResponse>()
            .await
            .map_err(|e| ApiError::ResponseError(e.to_string()))
    }

    /// Report token usage
    pub async fn report_usage(
        &self,
        clerk_token: &str,
        request: UsageReportRequest,
    ) -> Result<UsageReportResponse, ApiError> {
        let url = self.config.usage_report_url();

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", clerk_token))
            .json(&request)
            .send()
            .await
            .map_err(|e| ApiError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            
            if status == 401 {
                return Err(ApiError::AuthenticationError(text));
            }
            return Err(ApiError::ResponseError(format!(
                "Status {}: {}",
                status, text
            )));
        }

        response
            .json::<UsageReportResponse>()
            .await
            .map_err(|e| ApiError::ResponseError(e.to_string()))
    }

    /// Get usage history and current quota
    pub async fn get_usage_history(
        &self,
        clerk_token: &str,
    ) -> Result<UsageHistoryResponse, ApiError> {
        let url = self.config.usage_history_url();

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", clerk_token))
            .send()
            .await
            .map_err(|e| ApiError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            
            if status == 401 {
                return Err(ApiError::AuthenticationError(text));
            }
            return Err(ApiError::ResponseError(format!(
                "Status {}: {}",
                status, text
            )));
        }

        response
            .json::<UsageHistoryResponse>()
            .await
            .map_err(|e| ApiError::ResponseError(e.to_string()))
    }

    /// Verify Clerk token validity
    pub async fn verify_auth(&self, clerk_token: &str) -> Result<AuthVerifyResponse, ApiError> {
        let url = self.config.auth_verify_url();

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", clerk_token))
            .send()
            .await
            .map_err(|e| ApiError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            
            if status == 401 {
                return Err(ApiError::AuthenticationError(text));
            }
            return Err(ApiError::ResponseError(format!(
                "Status {}: {}",
                status, text
            )));
        }

        response
            .json::<AuthVerifyResponse>()
            .await
            .map_err(|e| ApiError::ResponseError(e.to_string()))
    }
}

impl Default for ApiClient {
    fn default() -> Self {
        Self::new()
    }
}

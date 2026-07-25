// Example: How to use the API client in a Tauri command

// This is a TEMPLATE for how to integrate license checking into existing commands.
// Add this pattern to your real commands in commands.rs

use crate::api_client::ApiClient;
use crate::api_types::{UsageReportRequest, LicenseCheckResponse};
use crate::settings::AppSettings;

/// Example: Run an analysis with license checking
/// 
/// This demonstrates the complete flow:
/// 1. Check if user is authenticated
/// 2. Verify license and token quota
/// 3. Run the analysis
/// 4. Report token usage
#[tauri::command]
pub async fn example_run_analysis_with_license(
    clerk_token: String,
    device_id: String,
    analysis_type: String,
) -> Result<String, String> {
    // Create API client (uses default dev/prod config)
    let api_client = ApiClient::new();

    // Step 1: Verify Clerk token is valid
    let auth_check = api_client
        .verify_auth(&clerk_token)
        .await
        .map_err(|e| format!("Auth verification failed: {}", e))?;

    if !auth_check.valid {
        return Err("Clerk token invalid or expired. Please re-authenticate.".to_string());
    }

    // Step 2: Check license and available quota
    let license: LicenseCheckResponse = api_client
        .check_license(&clerk_token)
        .await
        .map_err(|e| format!("License check failed: {}", e))?;

    if !license.valid {
        return Err("License invalid. Please check your subscription.".to_string());
    }

    // Assume this analysis will consume ~5000 tokens
    const TOKENS_NEEDED: i64 = 5000;
    
    if license.tokens_remaining < TOKENS_NEEDED {
        return Err(format!(
            "Insufficient token quota. You have {} tokens remaining, but this analysis needs {}. Please upgrade your plan.",
            license.tokens_remaining, TOKENS_NEEDED
        ));
    }

    // Step 3: Run the analysis
    // (Replace this with your actual analysis logic)
    let analysis_result = "Sample analysis result".to_string();
    let tokens_actually_used = 4523; // Actual tokens consumed by the AI

    // Step 4: Report usage back to Web API
    let usage_report = UsageReportRequest {
        tokens_consumed: tokens_actually_used,
        analysis_type: analysis_type.clone(),
        device_id,
        status: "success".to_string(),
    };

    api_client
        .report_usage(&clerk_token, usage_report)
        .await
        .map_err(|e| format!("Failed to report usage: {}", e))?;

    // Success!
    Ok(format!(
        "Analysis complete. Used {} tokens. {} tokens remaining.",
        tokens_actually_used,
        license.tokens_remaining - tokens_actually_used
    ))
}

/// Example: Get user's current quota status
#[tauri::command]
pub async fn example_get_usage_info(clerk_token: String) -> Result<String, String> {
    let api_client = ApiClient::new();

    let history = api_client
        .get_usage_history(&clerk_token)
        .await
        .map_err(|e| format!("Failed to get usage history: {}", e))?;

    Ok(format!(
        "Plan: {}\nTokens used this month: {}\nTokens remaining: {}",
        history.plan, history.tokens_this_month, history.tokens_remaining
    ))
}

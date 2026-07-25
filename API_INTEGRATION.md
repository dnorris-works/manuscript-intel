# API Integration Guide

This document describes how the Desktop app integrates with the Loremetry Web API for authentication and usage tracking.

## Architecture

```
Desktop App (Rust/Tauri)
    ↓
    ├─ api_client.rs      - HTTP client for Web API calls
    ├─ api_config.rs      - Configuration (URLs, timeouts)
    ├─ api_types.rs       - Request/response types
    └─ settings.rs        - User auth state & device ID
    ↓
Web API (Node.js/Express)
    ├─ /api/auth/*        - Authentication verification
    ├─ /api/license/*     - License checking
    └─ /api/usage/*       - Usage tracking
```

## Key Components

### 1. **API Client** (`api_client.rs`)

The `ApiClient` struct provides methods for communicating with the Web API:

- `check_license(clerk_token)` - Verify license validity and token quota
- `report_usage(clerk_token, request)` - Report token consumption
- `get_usage_history(clerk_token)` - Get usage stats
- `verify_auth(clerk_token)` - Verify Clerk token is valid

All methods require a Clerk JWT token in the Authorization header.

### 2. **Configuration** (`api_config.rs`)

The `ApiConfig` struct manages API URLs and timeouts:

- **Dev:** `http://localhost:3000`
- **Production:** `https://api.loremetry.com` (or `LOREMETRY_API_URL` env var)
- **Timeout:** 30 seconds
- **SSL verification:** Disabled in dev, enabled in production

### 3. **Settings** (`settings.rs`)

The `AppSettings` struct stores per-device state:

- `clerk_token` - User's JWT from Clerk
- `device_id` - Unique UUID for this desktop installation
- `user_email` - User's email from Clerk
- `api_url` - Configured API endpoint
- `last_sync` - Last successful sync timestamp

Settings are persisted to disk as JSON in the Tauri app data directory.

## Integration Workflow

### On App Launch

```rust
// Load settings from disk
let mut settings = AppSettings::load(&settings_path)?;

// Check if user is authenticated
if !settings.is_authenticated() {
    // Redirect to Clerk login (browser)
    // User completes auth flow
    // Desktop receives Clerk token
    settings.set_auth(clerk_token, user_email);
    settings.save(&settings_path)?;
}

// Create API client with config
let api_client = ApiClient::new();

// Verify auth token is still valid
let auth_response = api_client.verify_auth(&settings.clerk_token.unwrap()).await?;
if !auth_response.valid {
    // Token expired, need to re-authenticate
    settings.clear_auth();
    settings.save(&settings_path)?;
}
```

### Before Running an Analysis

```rust
// Check license quota
let license = api_client.check_license(&clerk_token).await?;

if !license.valid {
    return Err("License invalid - check subscription status");
}

if license.tokens_remaining < TOKENS_NEEDED {
    return Err("Insufficient token quota - upgrade plan");
}

// Proceed with analysis
let result = run_analysis(...)
```

### After Analysis Completes

```rust
// Report token consumption
let usage = UsageReportRequest {
    tokens_consumed: actual_tokens_used,
    analysis_type: "chapter_summary",
    device_id: settings.device_id.clone(),
    status: "success".to_string(),
};

api_client.report_usage(&clerk_token, usage).await?;
settings.last_sync = Some(chrono::Utc::now().to_rfc3339());
settings.save(&settings_path)?;
```

## Error Handling

The `ApiError` enum represents different failure modes:

- `RequestError` - Network/connection issues
- `ResponseError` - HTTP errors (500, 400, etc.)
- `AuthenticationError` - Invalid/expired Clerk token
- `QuotaExceeded` - User exceeded token quota

### Example

```rust
match api_client.check_license(&token).await {
    Ok(license) => {
        // Use license.valid and license.tokens_remaining
    }
    Err(ApiError::AuthenticationError(msg)) => {
        // Re-authenticate with Clerk
    }
    Err(ApiError::QuotaExceeded(msg)) => {
        // Show "upgrade plan" message to user
    }
    Err(e) => {
        // Handle other errors
        eprintln!("API error: {}", e);
    }
}
```

## Testing

### Dev Environment

```bash
# Start Web API
cd web-api
npm run dev

# Web API runs on http://localhost:3000

# In desktop app, ApiConfig will automatically use localhost:3000
# Set CLERK_SECRET_KEY in web-api/.env
# Set up test Clerk account for local testing
```

### Production Environment

1. Deploy web-api via Miget
2. Set `LOREMETRY_API_URL` environment variable in desktop build to production URL
3. Or set `LOREMETRY_API_URL=https://api.loremetry.com` in desktop app config

## Future Enhancements

- [ ] Offline mode with local queue for failed API calls
- [ ] Token refresh logic when expiring
- [ ] Rate limiting and retry logic
- [ ] Usage dashboard UI in desktop app
- [ ] Subscription management view (open web dashboard)

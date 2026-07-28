// ai.rs — Shared AI readiness checks for local and cloud providers

pub fn ai_ready(provider: &str, api_key: &str, model: &str) -> Result<(), String> {
    if model.trim().is_empty() {
        return Err("No model set. Go to Settings.".to_string());
    }
    if provider == "local" {
        if !crate::local_ai::is_ready() {
            return Err(
                "Local AI is not ready. Check Settings or wait for the model download.".to_string(),
            );
        }
        return Ok(());
    }
    if api_key.trim().is_empty() {
        return Err("No API key set. Go to Settings.".to_string());
    }
    Ok(())
}

// ai.rs — Shared AI readiness checks and provider routing

/// Resolved LLM credentials for a specific task.
pub struct LlmRoute {
    pub provider: String,
    pub api_key: String,
    pub model: String,
}

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

/// Genre classification, ranking, and category fit need a capable cloud model.
/// When the active provider is local, route to TokenMix using the stored API key.
pub fn route_for_genre_work(
    provider: &str,
    api_key: &str,
    model: &str,
    tokenmix_api_key: &str,
    genre_model: &str,
) -> Result<LlmRoute, String> {
    if provider == "local" {
        let key = if tokenmix_api_key.trim().is_empty() {
            api_key
        } else {
            tokenmix_api_key
        };
        if key.trim().is_empty() {
            return Err(
                "Genre analysis uses TokenMix when Local AI is selected. Add your TokenMix API key in Settings.".to_string(),
            );
        }
        let m = if genre_model.trim().is_empty() {
            model
        } else {
            genre_model
        };
        if m.trim().is_empty() {
            return Err("No genre model set. Assign a model to Genre in Settings.".to_string());
        }
        return Ok(LlmRoute {
            provider: "tokenmix".to_string(),
            api_key: key.to_string(),
            model: m.to_string(),
        });
    }

    if api_key.trim().is_empty() {
        return Err("No API key set. Go to Settings.".to_string());
    }
    let m = if genre_model.trim().is_empty() {
        model
    } else {
        genre_model
    };
    if m.trim().is_empty() {
        return Err("No model set. Go to Settings.".to_string());
    }
    Ok(LlmRoute {
        provider: provider.to_string(),
        api_key: api_key.to_string(),
        model: m.to_string(),
    })
}

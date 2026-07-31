// llm.rs — TokenMix client with prompt caching, usage telemetry, and shared call options.

use std::time::Duration;

use serde_json::{json, Value};

#[derive(Clone, Debug, Default)]
pub struct LlmCallOpts<'a> {
    /// Stable key so repeated prefixes (system + bible) hit provider prompt cache.
    pub cache_key:     Option<&'a str>,
    /// Template or feature id for usage logging.
    pub template_id:   Option<&'a str>,
}

#[derive(Clone, Debug, Default)]
pub struct LlmUsage {
    pub input_tokens:  u32,
    pub output_tokens: u32,
    pub cached_tokens: u32,
}

/// Build a stable cache key for per-story template runs.
pub fn story_cache_key(template_id: &str, story_folder: &str) -> String {
    format!("{template_id}:{story_folder}")
}

pub async fn call_llm(
    provider: &str,
    api_key: &str,
    model: &str,
    system: &str,
    user: &str,
    max_tokens: u32,
    opts: LlmCallOpts<'_>,
) -> Result<String, String> {
    call(provider, api_key, model, system, user, max_tokens, false, opts).await
}

pub async fn call_llm_json(
    provider: &str,
    api_key: &str,
    model: &str,
    system: &str,
    user: &str,
    max_tokens: u32,
    opts: LlmCallOpts<'_>,
) -> Result<String, String> {
    call(provider, api_key, model, system, user, max_tokens, true, opts).await
}

async fn call(
    provider: &str,
    api_key: &str,
    model: &str,
    system: &str,
    user: &str,
    max_tokens: u32,
    json_mode: bool,
    opts: LlmCallOpts<'_>,
) -> Result<String, String> {
    if provider != "tokenmix" {
        return Err(format!("Unsupported provider '{provider}'. TokenMix is required."));
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let mut last_err = String::new();
    for candidate in tokenmix_model_candidates(model) {
        let mut body = json!({
            "model": candidate,
            "max_tokens": max_tokens,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user}
            ],
            "cache_config": {
                "enabled": true,
                "ttl": 300
            }
        });

        if json_mode {
            body["response_format"] = json!({"type": "json_object"});
        }
        if let Some(key) = opts.cache_key {
            body["prompt_cache_key"] = json!(key);
        }

        let resp = client
            .post("https://api.tokenmix.ai/v1/chat/completions")
            .header("Authorization", format!("Bearer {api_key}"))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("TokenMix request failed: {e}"))?;

        let response: Value = resp
            .json()
            .await
            .map_err(|e| format!("TokenMix response parse failed: {e}"))?;

        if let Some(err) = response.get("error") {
            let msg = err["message"]
                .as_str()
                .unwrap_or_else(|| err.as_str().unwrap_or("unknown error"));
            let msg_lc = msg.to_ascii_lowercase();
            last_err = format!("TokenMix error: {msg}");
            if msg_lc.contains("model was not found") || msg_lc.contains("requested model was not found") {
                continue;
            }
            return Err(last_err);
        }

        let usage = parse_usage(&response);
        log_usage(opts.template_id, &candidate, &usage);

        if let Some(content) = response["choices"][0]["message"]["content"].as_str() {
            return Ok(content.to_string());
        }
        return Err("TokenMix: empty response".to_string());
    }

    if last_err.is_empty() {
        Err("TokenMix error: model not found for this API key.".to_string())
    } else {
        Err(last_err)
    }
}

fn parse_usage(response: &Value) -> LlmUsage {
    let usage = &response["usage"];
    let cached = usage["prompt_tokens_details"]["cached_tokens"]
        .as_u64()
        .or_else(|| usage["cached_tokens"].as_u64())
        .unwrap_or(0) as u32;
    LlmUsage {
        input_tokens:  usage["prompt_tokens"].as_u64().unwrap_or(0) as u32,
        output_tokens: usage["completion_tokens"].as_u64().unwrap_or(0) as u32,
        cached_tokens: cached,
    }
}

fn log_usage(template_id: Option<&str>, model: &str, usage: &LlmUsage) {
    if usage.input_tokens == 0 && usage.output_tokens == 0 {
        return;
    }
    let label = template_id.unwrap_or("llm");
    eprintln!(
        "[ai-usage] {label} model={model} in={} out={} cached={}",
        usage.input_tokens, usage.output_tokens, usage.cached_tokens
    );
}

fn tokenmix_model_candidates(model: &str) -> Vec<String> {
    let raw = model.trim();
    if raw.is_empty() {
        return Vec::new();
    }

    let mut candidates: Vec<String> = Vec::new();
    let mut push_unique = |v: &str| {
        let t = v.trim();
        if t.is_empty() {
            return;
        }
        if !candidates.iter().any(|c| c == t) {
            candidates.push(t.to_string());
        }
    };

    push_unique(raw);
    if let Some(slash) = raw.rfind('/') {
        push_unique(&raw[slash + 1..]);
    }
    if let Some(colon) = raw.rfind(':') {
        push_unique(&raw[colon + 1..]);
    }
    candidates
}

/// Compress writing-chat history: keep the last `keep_recent` turns verbatim,
/// replace older turns with a short rolling summary block.
pub fn compress_chat_history(
    history: &[(String, String)],
    keep_recent: usize,
) -> Vec<(String, String)> {
    if history.len() <= keep_recent {
        return history.to_vec();
    }

    let split = history.len().saturating_sub(keep_recent);
    let older = &history[..split];
    let recent = &history[split..];

    let mut summary_parts: Vec<String> = Vec::new();
    for (role, content) in older {
        let snippet: String = content.chars().take(120).collect();
        let suffix = if content.chars().count() > 120 { "…" } else { "" };
        summary_parts.push(format!("{role}: {snippet}{suffix}"));
    }

    let mut out = Vec::new();
    out.push((
        "system".to_string(),
        format!(
            "Earlier conversation summary ({} messages):\n{}",
            older.len(),
            summary_parts.join("\n")
        ),
    ));
    out.extend(recent.iter().cloned());
    out
}

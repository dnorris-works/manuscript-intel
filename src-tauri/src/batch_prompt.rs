// batch_prompt.rs — Batched per-chapter LLM calls with word-budget chunking and single-chapter fallback.
//
// Sends multiple chapters in one request (shared system prompt + bible) and expects a keyed JSON
// response: { "chapters": { "filename.md": { ... } } }

use std::collections::HashMap;

use crate::analysis::extract_json_object;
use crate::db::Db;
use crate::prompts;

/// Default total words of chapter text per batch (excluding bible + overhead).
pub const DEFAULT_WORD_BUDGET: usize = 12_000;
/// Tighter budget for craft/publish per-chapter checks (longer excerpts per chapter).
pub const CRAFT_BATCH_WORD_BUDGET: usize = 8_000;

#[derive(Clone, Debug)]
pub struct BatchChapterItem {
    pub file:  String,
    pub title: String,
    pub text:  String,
}

impl BatchChapterItem {
    pub fn word_count(&self) -> usize {
        self.text.split_whitespace().count()
    }
}

/// Split items into batches where the sum of chapter word counts stays within `budget`.
pub fn chunk_by_word_budget(items: &[BatchChapterItem], budget: usize) -> Vec<Vec<usize>> {
    let budget = budget.max(1);
    let mut chunks: Vec<Vec<usize>> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    let mut current_words = 0usize;

    for (i, item) in items.iter().enumerate() {
        let words = item.word_count().max(1);
        if !current.is_empty() && current_words + words > budget {
            chunks.push(current);
            current = Vec::new();
            current_words = 0;
        }
        current.push(i);
        current_words += words;
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

/// Human-readable block listing chapters for the batch user message.
pub fn format_chapters_block(items: &[BatchChapterItem]) -> String {
    let mut out = String::new();
    for item in items {
        out.push_str(&format!(
            "=== FILE: {} ===\nTitle: {}\nWords: {}\n\n{}\n\n",
            item.file,
            item.title,
            item.word_count(),
            item.text
        ));
    }
    out.trim_end().to_string()
}

fn scaled_max_tokens(base: u32, chapter_count: usize) -> u32 {
    let scaled = base.saturating_mul(chapter_count as u32);
    scaled.clamp(base, 16_000)
}

/// Execute a batch prompt template with `{bible}`, `{chapters_block}`, and `{chapter_count}` filled.
pub async fn execute_batch_prompt(
    db: &Db,
    template_id: &str,
    provider: &str,
    api_key: &str,
    model: &str,
    bible: &str,
    items: &[BatchChapterItem],
) -> Result<String, String> {
    let chapters_block = format_chapters_block(items);
    let chapter_count = items.len().to_string();
    let mut vars = HashMap::new();
    vars.insert("bible", bible);
    vars.insert("chapters_block", chapters_block.as_str());
    vars.insert("chapter_count", chapter_count.as_str());

    let template = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        prompts::load_template(&conn, template_id)?
    };
    let max_tokens = scaled_max_tokens(template.max_tokens, items.len());

    let system_prompt = prompts::fill_template(&template.system_prompt, &vars);
    let user_content = prompts::fill_template(&template.user_template, &vars);

    if template.json_mode {
        crate::commands::call_llm_json(provider, api_key, model, &system_prompt, &user_content, max_tokens).await
    } else {
        crate::commands::call_llm(provider, api_key, model, &system_prompt, &user_content, max_tokens).await
    }
}

/// Execute a single-chapter prompt (fallback path).
pub async fn execute_single_chapter_prompt(
    db: &Db,
    template_id: &str,
    provider: &str,
    api_key: &str,
    model: &str,
    bible: &str,
    item: &BatchChapterItem,
    extra_vars: &[(&str, &str)],
) -> Result<String, String> {
    let mut vars = HashMap::new();
    let computed = format!("Word count: {}", item.word_count());
    vars.insert("chapter_title", item.title.as_str());
    vars.insert("chapter_text", item.text.as_str());
    vars.insert("bible", bible);
    vars.insert("computed_signals", computed.as_str());
    for (key, value) in extra_vars {
        vars.insert(*key, *value);
    }
    prompts::execute_prompt(db, template_id, provider, api_key, model, vars).await
}

/// Normalize a single-chapter LLM response into the standard per-chapter JSON shape.
pub fn wrap_single_response(
    template_id: &str,
    raw: &str,
) -> Result<serde_json::Value, String> {
    let clean = raw.trim()
        .trim_start_matches("```json").trim_start_matches("```")
        .trim_end_matches("```").trim();

    match template_id {
        "chapter_summary" => {
            if clean.is_empty() {
                return Err("Empty summary returned.".into());
            }
            Ok(serde_json::json!({ "summary": clean }))
        }
        "sdt_check" | "ai_isms_check" => {
            let findings = parse_json_array(clean)?;
            Ok(serde_json::json!({ "findings": findings }))
        }
        "continuity_extract" => {
            let facts = parse_json_array(clean)?;
            Ok(serde_json::json!({ "facts": facts }))
        }
        _ => {
            let obj = extract_json_object(clean).unwrap_or_else(|| clean.to_string());
            serde_json::from_str(&obj)
                .map_err(|e| format!("Parse error: {} | {}", e, &obj[..obj.len().min(200)]))
        }
    }
}

fn parse_json_array(clean: &str) -> Result<Vec<serde_json::Value>, String> {
    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(clean) {
        return Ok(arr);
    }
    let json_str = if clean.starts_with('[') {
        clean.to_string()
    } else if let Some(start) = clean.find('[') {
        extract_bracketed_array(clean, start)
    } else {
        clean.to_string()
    };
    serde_json::from_str(&json_str)
        .map_err(|e| format!("Array parse error: {} | {}", e, &json_str[..json_str.len().min(200)]))
}

fn extract_bracketed_array(clean: &str, start: usize) -> String {
    let bytes = clean.as_bytes();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    let mut end = clean.len();
    for (i, &b) in bytes[start..].iter().enumerate() {
        if escape {
            escape = false;
            continue;
        }
        match b {
            b'\\' if in_string => escape = true,
            b'"' => in_string = !in_string,
            b'[' if !in_string => depth += 1,
            b']' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    end = start + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    clean[start..end].to_string()
}

/// Run batched calls with per-chapter fallback when a batch fails or omits files.
pub async fn process_chapters_batched(
    db: &Db,
    provider: &str,
    api_key: &str,
    model: &str,
    batch_template_id: &str,
    single_template_id: &str,
    bible: &str,
    items: Vec<BatchChapterItem>,
    word_budget: usize,
    single_extra: &[(&str, &str)],
) -> HashMap<String, serde_json::Value> {
    if items.is_empty() {
        return HashMap::new();
    }

    if items.len() == 1 {
        let item = &items[0];
        return match run_single(db, provider, api_key, model, single_template_id, bible, item, single_extra).await {
            Ok(v) => HashMap::from([(item.file.clone(), v)]),
            Err(_) => HashMap::new(),
        };
    }

    let chunks = chunk_by_word_budget(&items, word_budget);
    let mut results: HashMap<String, serde_json::Value> = HashMap::new();

    for chunk_indices in chunks {
        if crate::is_cancelled() {
            break;
        }

        let chunk: Vec<BatchChapterItem> = chunk_indices.iter().map(|&i| items[i].clone()).collect();

        let batch_result = execute_batch_prompt(
            db,
            batch_template_id,
            provider,
            api_key,
            model,
            bible,
            &chunk,
        )
        .await;

        let parsed = match batch_result {
            Ok(raw) => parse_batch_chapters_map(&raw),
            Err(_) => Err("batch call failed".to_string()),
        };

        match parsed {
            Ok(map) if chunk.iter().all(|it| map.contains_key(&it.file)) => {
                results.extend(map);
            }
            Ok(map) => {
                for item in &chunk {
                    if let Some(value) = map.get(&item.file) {
                        results.insert(item.file.clone(), value.clone());
                    } else if let Ok(value) =
                        run_single(db, provider, api_key, model, single_template_id, bible, item, single_extra).await
                    {
                        results.insert(item.file.clone(), value);
                    }
                }
            }
            Err(_) => {
                for item in &chunk {
                    if let Ok(value) =
                        run_single(db, provider, api_key, model, single_template_id, bible, item, single_extra).await
                    {
                        results.insert(item.file.clone(), value);
                    }
                }
            }
        }
    }

    results
}

async fn run_single(
    db: &Db,
    provider: &str,
    api_key: &str,
    model: &str,
    single_template_id: &str,
    bible: &str,
    item: &BatchChapterItem,
    single_extra: &[(&str, &str)],
) -> Result<serde_json::Value, String> {
    let raw = execute_single_chapter_prompt(
        db,
        single_template_id,
        provider,
        api_key,
        model,
        bible,
        item,
        single_extra,
    )
    .await?;
    wrap_single_response(single_template_id, &raw)
}

/// Parse `{"chapters": {"file.md": {...}}}` from model output.
pub fn parse_batch_chapters_map(raw: &str) -> Result<HashMap<String, serde_json::Value>, String> {
    let clean = extract_json_object(raw).unwrap_or_else(|| raw.trim().to_string());
    let root: serde_json::Value = serde_json::from_str(&clean)
        .map_err(|e| format!("Batch JSON parse error: {} | {}", e, &clean[..clean.len().min(200)]))?;

    let chapters = root
        .get("chapters")
        .and_then(|v| v.as_object())
        .ok_or_else(|| "Batch response missing \"chapters\" object.".to_string())?;

    Ok(chapters.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
}

/// Extract a string field from a per-chapter batch value object.
pub fn chapter_string_field(value: &serde_json::Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Extract a JSON array field from a per-chapter batch value object.
pub fn chapter_array_field(value: &serde_json::Value, field: &str) -> Vec<serde_json::Value> {
    value
        .get(field)
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_by_word_budget_splits_heavy_chapters() {
        let items = vec![
            BatchChapterItem { file: "a.md".into(), title: "A".into(), text: "word ".repeat(5000) },
            BatchChapterItem { file: "b.md".into(), title: "B".into(), text: "word ".repeat(5000) },
            BatchChapterItem { file: "c.md".into(), title: "C".into(), text: "word ".repeat(1000) },
        ];
        let chunks = chunk_by_word_budget(&items, 6000);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0], vec![0]);
        assert_eq!(chunks[1], vec![1, 2]);
    }

    #[test]
    fn parse_batch_chapters_map_reads_file_keys() {
        let raw = r#"{"chapters":{"01.md":{"summary":"Romance"},"02.md":{"summary":"Suspense"}}}"#;
        let map = parse_batch_chapters_map(raw).unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(
            chapter_string_field(&map["01.md"], "summary").as_deref(),
            Some("Romance")
        );
    }
}

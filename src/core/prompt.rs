//! Prompt engineering and system prompt building.

use once_cell::sync::Lazy;

/// System prompt template with language placeholder.
static SYSTEM_PROMPT_TEMPLATE: Lazy<String> =
    Lazy::new(|| include_str!("../../prompts/system.txt").to_string());

/// Build the system prompt with the given language context.
pub fn build_system_prompt(lang: &str) -> String {
    // TODO: Load language-specific system prompt from templates
    // For now, use the template with language placeholder
    SYSTEM_PROMPT_TEMPLATE.replace("{LANG}", lang).replace(
        "{DATE}",
        &chrono::Local::now().format("%Y-%m-%d").to_string(),
    )
}

/// Build the full prompt for the LLM including conversation history.
pub fn build_full_prompt(
    system_prompt: &str,
    messages: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    let mut prompt_messages = Vec::with_capacity(messages.len() + 1);
    prompt_messages.push(serde_json::json!({
        "role": "system",
        "content": system_prompt
    }));
    prompt_messages.extend_from_slice(messages);
    prompt_messages
}

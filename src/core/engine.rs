use crate::core::memory::{ConversationMemory, Message, ToolCall};
use crate::core::router::RouterConfig;
use anyhow::Result;
use serde_json::Value;

use crate::core::storage::MemoryStorage;
use crate::core::tools::Tool;
use std::sync::Arc;

/// Enum for LLM responses, either plain text or tool calls.
#[derive(Debug)]
pub enum LLMResponse {
    Text(String),
    ToolCalls(Vec<ToolCall>),
}

/// Configuration for the LLM engine.
pub struct EngineConfig {
    pub api_url: String,
    pub api_key: String,
    pub model: String,
    pub model_pro: Option<String>,
    pub model_flash: Option<String>,
    pub max_tokens: u32,
    pub temperature: f32,
    pub storage: Arc<MemoryStorage>,
    pub allowed_paths: Arc<tokio::sync::RwLock<Vec<String>>>,
    pub router: Option<RouterConfig>,
}

/// Main LLM engine struct.
pub struct Engine {
    config: EngineConfig,
    client: reqwest::Client,
    pub storage: Arc<MemoryStorage>,
    pub allowed_paths: Arc<tokio::sync::RwLock<Vec<String>>>,
}

impl Engine {
    /// Access the engine configuration (for model routing).
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }
    /// Create a new engine with the given configuration.
    pub fn new(config: EngineConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()?;
        let storage = config.storage.clone();
        let allowed_paths = config.allowed_paths.clone();
        Ok(Self {
            config,
            client,
            storage,
            allowed_paths,
        })
    }

    /// Generate a response based on conversation history and optional tools for a specific user.
    /// `model_override` allows using a different model per call (e.g. pro for turn 0, flash for follow-ups).
    pub async fn generate(
        &self,
        user_id: i64,
        memory: &ConversationMemory,
        tools: &[Tool],
        _lang: &str,
        user_timezone: Option<&str>,
        model_override: Option<&str>,
    ) -> Result<LLMResponse> {
        let memory_context = self.storage.build_context_string(user_id);
        let current_time = chrono::Local::now()
            .format("%Y-%m-%d %H:%M:%S %Z")
            .to_string();

        let tz_info = match user_timezone {
            Some(tz) => format!(". User timezone: {}", tz),
            None => String::new(),
        };

        let system_prompt = format!(
            "You are ClavaMea, a helpful AI assistant. Always reply in the same language the user uses.\n\
            You have access to tools — use them when the user asks for an action.\n\
            - schedule_reminder: for simple reminders without internet search\n\
- schedule_web_search: for recurring reminders that search the web when triggered (sports scores, news, etc.)\n\
            - list_schedules: to list active reminders\n\
            - cancel_schedule: to cancel a reminder\n\
            - web_search: for current information\n\
            - file_reader / list_dir: for files and directories\n\
            - edit_code: to create or modify files\n\
            - set_user_timezone: to set the user's timezone\n\
            Only call a tool when the user explicitly requests the corresponding action.\n\
            For casual conversation or questions, reply with text normally.\n\
            DO NOT use Markdown tables — use bulleted lists or bold text instead.\n\
            \n\
            The current time is: {}{}.\n\n\
            {}",
            current_time, tz_info, memory_context
        );

        let mut msgs = vec![serde_json::json!({
            "role": "system",
            "content": system_prompt
        })];

        // If tools are available, add a short tool reminder as a second system message.
        // This sits right before the conversation history and helps override
        // any hallucination patterns from past assistant responses.
        if !tools.is_empty() {
            msgs.push(serde_json::json!({
                "role": "system",
                "content": "TOOLS ARE AVAILABLE. You MUST call the appropriate tool for any action the user requests. Do NOT reply with text claiming an action was performed — always call the tool first. After a tool succeeds, summarize the result — do NOT call more tools for the same request."
            }));
        }

        msgs.extend(memory.to_api_messages());

        let model = if let Some(router_config) = &self.config.router {
            if model_override.is_some() {
                model_override.unwrap_or(&self.config.model).to_string()
            } else {
                let prompt_len = msgs
                    .iter()
                    .filter_map(|m| m.get("content").and_then(|c| c.as_str()))
                    .map(|s| s.len())
                    .sum::<usize>();
                let tool_count = tools.len();
                let request_type = crate::core::router::analyze_request(prompt_len, tool_count, 0);
                router_config.select_model(request_type).to_string()
            }
        } else {
            model_override.unwrap_or(&self.config.model).to_string()
        };

        let mut payload = serde_json::json!({
            "model": model,
            "messages": msgs,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
            "thinking": { "type": "disabled" },
            "tool_choice": "auto",
        });

        // Add tools if available
        if !tools.is_empty() {
            let tool_definitions: Vec<Value> = tools.iter().map(|t| t.definition()).collect();
            payload["tools"] = serde_json::json!(tool_definitions);
        }

        // Determine API endpoint and key based on router configuration
        let (api_url, api_key) = if let Some(router_config) = &self.config.router {
            (
                "https://openrouter.ai/api/v1/chat/completions".to_string(),
                router_config.api_key.clone(),
            )
        } else {
            (self.config.api_url.clone(), self.config.api_key.clone())
        };

        // Build endpoint URL
        let mut endpoint = api_url.clone();
        if !endpoint.ends_with("/chat/completions") {
            if endpoint.ends_with("/") {
                endpoint.push_str("chat/completions");
            } else {
                endpoint.push_str("/chat/completions");
            }
        }

        // Make API request with fallback on 429 + exponential backoff
        let res = if let Some(router_config) = &self.config.router {
            let models = &router_config.models;
            let mut result_res: Option<reqwest::Response> = None;
            let base_timeout = std::time::Duration::from_secs(15);

            for (i, model_attempt) in models.iter().enumerate() {
                payload["model"] = serde_json::json!(model_attempt);

                let attempt_timeout = base_timeout * 2u32.pow(i as u32);
                let client = reqwest::Client::builder()
                    .timeout(attempt_timeout)
                    .connect_timeout(std::time::Duration::from_secs(10))
                    .build()?;

                let res = client
                    .post(&endpoint)
                    .bearer_auth(&api_key)
                    .json(&payload)
                    .send()
                    .await?;

                if res.status() == 429 {
                    tracing::warn!("Rate limited on model {}, trying next", model_attempt);
                    let err_msg = format!("Rate limited on model {}", model_attempt);
                    if i + 1 < models.len() {
                        continue;
                    } else {
                        return Err(anyhow::anyhow!("Rate limited on all models: {}", err_msg));
                    }
                }

                if !res.status().is_success() {
                    let status = res.status();
                    let text = res.text().await.unwrap_or_default();
                    tracing::error!("LLM API error {}: {}", status, text);
                    return Err(anyhow::anyhow!("LLM API error {}: {}", status, text));
                }

                result_res = Some(res);
                break;
            }
            result_res.unwrap()
        } else {
            self.client
                .post(&endpoint)
                .bearer_auth(&api_key)
                .json(&payload)
                .send()
                .await?
        };

        let res_text = res.text().await.unwrap_or_default();
        tracing::debug!("LLM raw response: {}", res_text);

        let data: Value = serde_json::from_str(&res_text).map_err(|e| {
            anyhow::anyhow!("Failed to parse LLM response: {} | body: {}", e, res_text)
        })?;
        let message = &data["choices"][0]["message"];

        if let Some(tool_calls) = message["tool_calls"].as_array() {
            let calls: Vec<ToolCall> = serde_json::from_value(serde_json::json!(tool_calls))?;
            return Ok(LLMResponse::ToolCalls(calls));
        }

        let content = message["content"]
            .as_str()
            .unwrap_or("Sorry, I could not generate a response.")
            .to_string();

        Ok(LLMResponse::Text(content))
    }

    /// Generate a response with tool calling support.
    pub async fn generate_with_tools(
        &self,
        _memory: &ConversationMemory,
        _lang: &str,
        _tools: &[Value],
    ) -> Result<Value> {
        // TODO: Implement tool calling
        Ok(Value::Null)
    }
}

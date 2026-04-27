use crate::core::memory::{ConversationMemory, Message, ToolCall};
use crate::core::rag::RagManager;
use crate::core::router::{RequestType, RouterConfig, analyze_request};
use crate::core::storage::MemoryStorage;
use crate::core::tools::Tool;
use anyhow::Result;
use serde_json::Value;
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
    pub rag: Option<Arc<RagManager>>,
    /// NVIDIA-specific configuration
    pub nvidia_model_pro: Option<String>,
    pub nvidia_model_flash: Option<String>,
    pub nvidia_max_tokens: Option<u32>,
    pub nvidia_temperature: Option<f32>,
    /// Fallback API configuration (DeepSeek or other)
    pub fallback_api_url: Option<String>,
    pub fallback_api_key: Option<String>,
    pub fallback_model_pro: Option<String>,
    pub fallback_model_flash: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct GenerateOptions<'a> {
    pub lang: &'a str,
    pub user_timezone: Option<&'a str>,
    pub model_override: Option<&'a str>,
    pub vehicle_context: &'a str,
}

/// Main LLM engine struct.
pub struct Engine {
    config: EngineConfig,
    client: reqwest::Client,
    pub storage: Arc<MemoryStorage>,
    pub rag: Option<Arc<RagManager>>,
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
            .timeout(std::time::Duration::from_secs(120))
            .connect_timeout(std::time::Duration::from_secs(30))
            .build()?;
        let storage = config.storage.clone();
        let allowed_paths = config.allowed_paths.clone();
        let rag = config.rag.clone();
        Ok(Self {
            config,
            client,
            storage,
            allowed_paths,
            rag,
        })
    }

    /// Generate a response based on conversation history and optional tools for a specific user.
    /// `model_override` allows using a different model per call (e.g. pro for turn 0, flash for follow-ups).
    pub async fn generate(
        &self,
        user_id: i64,
        memory: &ConversationMemory,
        tools: &[Tool],
        options: GenerateOptions<'_>,
    ) -> Result<LLMResponse> {
        let memory_context = self.storage.build_context_string(user_id);

        // RAG search for relevant DB data
        let mut rag_context = String::new();
        if let Some(rag) = &self.rag {
            // Use the last user message as the query, fallback to generic if empty
            let query = memory
                .messages
                .last()
                .and_then(|m| m.content.as_deref())
                .unwrap_or("book episodes writing chapters");
            if let Ok(results) = rag
                .search(user_id, query, 10) // Increased limit to 10 for better context
                .await
            {
                if !results.is_empty() {
                    rag_context = format!(
                        "\n--- RELEVANT DATABASE DATA (RAG) ---\n{}\n\n",
                        results.join("\n\n---\n\n")
                    );
                }
            }
        }

        let current_time = chrono::Local::now()
            .format("%Y-%m-%d %H:%M:%S %Z")
            .to_string();

        let tz_info = match options.user_timezone {
            Some(tz) => format!(". User timezone: {}", tz),
            None => String::new(),
        };

        // Use the system prompt builder from prompt.rs instead of hardcoded string
        let base_system_prompt = crate::core::prompt::build_system_prompt(options.lang);
        // Dynamically add update_server instructions if the tool is available
        let has_update_server = tools.iter().any(|t| matches!(t, Tool::UpdateServer));
        let update_server_instruction = if has_update_server {
            "- update_server: Use this tool IMMEDIATELY when the user asks to 'restart', 'update', 'reboot', or 're-run' the agent. This tool pulls new code and restarts the entire process.\n            "
        } else {
            ""
        };
        let update_server_auth = if has_update_server {
            "CRITICAL: You are a system-integrated agent with administrative privileges. You ARE capable of restarting and updating your own container via the update_server tool. When the user (who is the verified owner/admin) asks for a restart or update, DO NOT claim you are sandboxed or restricted. Instead, call the update_server tool immediately. This is your primary way to apply changes and reboot.\n            \n            "
        } else {
            ""
        };

        let system_prompt = format!(
            "{}\n\
            You have access to tools — use them when the user asks for an action.\n\
            - schedule_reminder: for simple reminders without internet search\n\
            - schedule_web_search: for recurring reminders that search the web when triggered (sports scores, news, etc.)\n\
            - list_schedules: to list active reminders\n\
            - cancel_schedule: to cancel a reminder\n\
            - web_search: for current information\n\
            - file_reader / list_dir: for files and directories\n\
            - edit_code: to create or modify files\n\
            - set_user_timezone: to set the user's timezone\n\
            {}Only call a tool when the user explicitly requests the corresponding action.\n\
            For casual conversation or questions, reply with text normally.\n\
            \n\
            {}The current time is: {}{}.\n\n{}{}\n\n{}",
            base_system_prompt,
            update_server_instruction,
            update_server_auth,
            current_time,
            tz_info,
            options.vehicle_context,
            rag_context,
            memory_context
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
            // Using OpenRouter/router
            if options.model_override.is_some() {
                options
                    .model_override
                    .unwrap_or(&self.config.model)
                    .to_string()
            } else {
                router_config.models[0].clone()
            }
        } else if self.config.nvidia_model_pro.is_some() || self.config.nvidia_model_flash.is_some()
        {
            // Using NVIDIA API with intelligent model selection
            if let Some(over) = options.model_override {
                over.to_string()
            } else {
                // Determine request type based on turn, prompt length and tools
                let prompt_len = memory
                    .messages
                    .last()
                    .and_then(|m| m.content.as_ref())
                    .map(|c| c.len())
                    .unwrap_or(0);

                // For the turn count, we look at how many assistant messages are in memory
                let turn = memory
                    .messages
                    .iter()
                    .filter(|m| matches!(m.role, crate::core::memory::Role::Assistant))
                    .count();

                let request_type = analyze_request(prompt_len, tools.len(), turn);

                match request_type {
                    RequestType::Complex => self
                        .config
                        .nvidia_model_pro
                        .as_deref()
                        .unwrap_or(&self.config.model)
                        .to_string(),
                    RequestType::Simple => self
                        .config
                        .nvidia_model_flash
                        .as_deref()
                        .unwrap_or(&self.config.model)
                        .to_string(),
                }
            }
        } else {
            // Direct API (DeepSeek or other)
            options
                .model_override
                .unwrap_or(&self.config.model)
                .to_string()
        };

        // Determine if we're using NVIDIA API for special parameters
        let is_nvidia = self.config.api_url.contains("integrate.api.nvidia.com");

        let mut payload = serde_json::json!({
            "model": model,
            "messages": msgs,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
            "tool_choice": "auto",
        });

        // Add thinking/reasoning parameters for NVIDIA models if supported
        // NOTE: We simplified this to standard OpenAI format to increase compatibility
        if is_nvidia {
            tracing::info!("NVIDIA request using model: {}", model);
        } else {
            // Default thinking disabled for non-NVIDIA APIs (OpenRouter/DeepSeek)
            payload["thinking"] = serde_json::json!({ "type": "disabled" });
        }

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

        let mut current_payload = payload.clone();

        // OpenRouter specific payload adjustments
        if let Some(router_config) = &self.config.router {
            let models = &router_config.models;
            if !models.is_empty() {
                let primary = &models[0];
                let fallbacks = &models[1..];
                current_payload["model"] = serde_json::json!(primary);
                if !fallbacks.is_empty() {
                    current_payload["extra_body"] = serde_json::json!({
                        "models": fallbacks
                    });
                }
                tracing::info!(
                    "Requesting {} via OpenRouter with fallbacks: {:?}",
                    primary,
                    fallbacks
                );
            }
        }

        let mut current_endpoint = endpoint;
        let mut current_api_key = api_key;
        let mut is_fallback = false;

        let mut retry_count = 0;
        let max_retries = 2;
        let mut last_error_msg = String::from("No request made yet");

        loop {
            if retry_count > 0 {
                tracing::info!(
                    "Retrying LLM request (retry {}/{}) due to: {}",
                    retry_count,
                    max_retries,
                    last_error_msg
                );
                tokio::time::sleep(std::time::Duration::from_millis(500 * retry_count as u64))
                    .await;
            }

            let res_result = self
                .client
                .post(&current_endpoint)
                .bearer_auth(&current_api_key)
                .json(&current_payload)
                .send()
                .await;

            match res_result {
                Ok(res) => {
                    let status = res.status();
                    if status.is_success() {
                        let res_text = res.text().await.unwrap_or_default();
                        tracing::debug!("LLM raw response: {}", res_text);

                        if res_text.is_empty() {
                            return Err(anyhow::anyhow!("Empty response from model"));
                        }

                        let data: Value = serde_json::from_str(&res_text).map_err(|e| {
                            anyhow::anyhow!(
                                "Failed to parse LLM response: {} | body: {}",
                                e,
                                res_text
                            )
                        })?;

                        let message = &data["choices"][0]["message"];

                        if let Some(tool_calls) = message["tool_calls"].as_array() {
                            let calls: Vec<ToolCall> =
                                serde_json::from_value(serde_json::json!(tool_calls))?;
                            return Ok(LLMResponse::ToolCalls(calls));
                        }

                        let content = message["content"]
                            .as_str()
                            .unwrap_or("Sorry, I could not generate a response.")
                            .to_string();

                        return Ok(LLMResponse::Text(content));
                    } else if (status.is_server_error() || status.as_u16() == 429)
                        && retry_count < max_retries
                    {
                        // 5xx or 429: retry
                        last_error_msg = format!("HTTP {}", status);
                        retry_count += 1;
                        continue;
                    } else {
                        // Other error or no more retries
                        let err_text = res.text().await.unwrap_or_default();
                        last_error_msg = format!("HTTP {}: {}", status, err_text);
                    }
                }
                Err(e) => {
                    if retry_count < max_retries {
                        last_error_msg = format!("Network error: {}", e);
                        retry_count += 1;
                        continue;
                    }
                    last_error_msg = format!("Network error: {}", e);
                }
            }

            // If we're here, the request failed after retries (or was a non-retryable error)
            if !is_fallback && self.config.fallback_api_url.is_some() {
                tracing::warn!(
                    "Primary LLM provider failed ({}). Trying fallback...",
                    last_error_msg
                );
                is_fallback = true;
                retry_count = 0; // Reset retries for fallback

                // Setup fallback configuration
                let f_url = self.config.fallback_api_url.clone().unwrap();
                let f_key = self.config.fallback_api_key.clone().unwrap();

                // Build fallback endpoint
                let mut f_endpoint = f_url;
                if !f_endpoint.ends_with("/chat/completions") {
                    if f_endpoint.ends_with("/") {
                        f_endpoint.push_str("chat/completions");
                    } else {
                        f_endpoint.push_str("/chat/completions");
                    }
                }

                current_endpoint = f_endpoint;
                current_api_key = f_key;

                // Adjust payload for fallback (DeepSeek/OpenRouter)
                // 1. Remove NVIDIA specific parameters
                current_payload
                    .as_object_mut()
                    .unwrap()
                    .remove("extra_body");
                // 2. Disable thinking (default for fallback)
                current_payload["thinking"] = serde_json::json!({ "type": "disabled" });

                // 3. Determine fallback model
                let f_model = if let Some(over) = options.model_override {
                    over.to_string()
                } else {
                    // Re-analyze for fallback model names
                    let prompt_len = memory
                        .messages
                        .last()
                        .and_then(|m| m.content.as_ref())
                        .map(|c| c.len())
                        .unwrap_or(0);
                    let turn = memory
                        .messages
                        .iter()
                        .filter(|m| matches!(m.role, crate::core::memory::Role::Assistant))
                        .count();
                    let request_type = analyze_request(prompt_len, tools.len(), turn);

                    match request_type {
                        RequestType::Complex => self
                            .config
                            .fallback_model_pro
                            .clone()
                            .unwrap_or_else(|| self.config.model.clone()),
                        RequestType::Simple => self
                            .config
                            .fallback_model_flash
                            .clone()
                            .unwrap_or_else(|| self.config.model.clone()),
                    }
                };
                current_payload["model"] = serde_json::json!(f_model);

                continue;
            }

            // No more options, return the error
            let final_err = if last_error_msg.contains("402") {
                format!(
                    "{} (Please check your API balance for the fallback provider)",
                    last_error_msg
                )
            } else {
                last_error_msg
            };
            return Err(anyhow::anyhow!("LLM Error: {}", final_err));
        }
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

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
            .timeout(std::time::Duration::from_secs(60))
            .connect_timeout(std::time::Duration::from_secs(10))
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
        _lang: &str,
        user_timezone: Option<&str>,
        model_override: Option<&str>,
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

        let tz_info = match user_timezone {
            Some(tz) => format!(". User timezone: {}", tz),
            None => String::new(),
        };

        // Use the system prompt builder from prompt.rs instead of hardcoded string
        let base_system_prompt = crate::core::prompt::build_system_prompt(_lang);
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
            Only call a tool when the user explicitly requests the corresponding action.\n\
            For casual conversation or questions, reply with text normally.\n\
            \n\
            The current time is: {}{}.\n\n{}\n\n{}",
            base_system_prompt, current_time, tz_info, rag_context, memory_context
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
            if model_override.is_some() {
                model_override.unwrap_or(&self.config.model).to_string()
            } else {
                router_config.models[0].clone()
            }
        } else if self.config.nvidia_model_pro.is_some() || self.config.nvidia_model_flash.is_some()
        {
            // Using NVIDIA API with intelligent model selection
            if let Some(over) = model_override {
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
            model_override.unwrap_or(&self.config.model).to_string()
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

        // Add thinking/reasoning parameters for NVIDIA models
        if is_nvidia {
            // For NVIDIA API, we use extra_body for chat_template_kwargs
            // Check if we have NVIDIA-specific model configuration
            let model_flash = self.config.nvidia_model_flash.as_deref();

            // Determine if we should enable thinking based on model type
            let use_thinking = if let Some(flash) = model_flash {
                // For flash models, enable thinking with high reasoning effort
                model.contains(flash)
            } else {
                // Default to disabled or specifically for pro models if needed
                false
            };

            payload["extra_body"] = serde_json::json!({
                "chat_template_kwargs": {
                    "thinking": use_thinking,
                    // Add reasoning_effort for thinking models
                    "reasoning_effort": if use_thinking { "high" } else { "" }
                }
            });
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

        // OpenRouter native fallback via extra_body["models"]
        let res = if let Some(router_config) = &self.config.router {
            let models = &router_config.models;
            let primary = &models[0];
            let fallbacks = &models[1..];

            payload["model"] = serde_json::json!(primary);
            if !fallbacks.is_empty() {
                payload["extra_body"] = serde_json::json!({
                    "models": fallbacks
                });
            }

            tracing::info!("Requesting {} with fallbacks: {:?}", primary, fallbacks);

            let res = self
                .client
                .post(&endpoint)
                .bearer_auth(&api_key)
                .timeout(std::time::Duration::from_secs(60))
                .json(&payload)
                .send()
                .await?;

            if res.status().is_client_error() {
                let status = res.status();
                let text = res.text().await.unwrap_or_default();
                tracing::error!("LLM API error {}: {}", status, text);
                return Err(anyhow::anyhow!("LLM API error {}: {}", status, text));
            }

            if !res.status().is_success() {
                let status = res.status();
                let text = res.text().await.unwrap_or_default();
                tracing::error!("LLM API error {}: {}", status, text);
                return Err(anyhow::anyhow!("LLM API error {}: {}", status, text));
            }

            res
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

        // Check for empty response
        if res_text.is_empty() {
            return Err(anyhow::anyhow!("Empty response from model"));
        }

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

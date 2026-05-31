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

impl LLMResponse {
    fn is_empty(&self) -> bool {
        match self {
            LLMResponse::Text(s) => s.is_empty(),
            LLMResponse::ToolCalls(tc) => tc.is_empty(),
        }
    }
}

/// Configuration for a single LLM provider.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub api_url: String,
    pub api_key: String,
    pub model_pro: Option<String>,
    pub model_flash: Option<String>,
    pub max_tokens: u32,
    pub temperature: f32,
    pub router: Option<RouterConfig>,
    /// NVIDIA-specific configuration
    pub nvidia_model_pro: Option<String>,
    pub nvidia_model_flash: Option<String>,
    pub nvidia_max_tokens: Option<u32>,
    pub nvidia_temperature: Option<f32>,
}

/// Configuration for the LLM engine.
pub struct EngineConfig {
    pub model: String,
    pub model_pro: Option<String>,
    pub model_flash: Option<String>,
    pub max_tokens: u32,
    pub temperature: f32,
    pub storage: Arc<MemoryStorage>,
    pub allowed_paths: Arc<tokio::sync::RwLock<Vec<String>>>,
    pub rag: Option<Arc<RagManager>>,
    pub providers: Vec<ProviderConfig>,
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
            The current time is: {}{}.\n\n{}{}\n\n{}\n\n\
            ### ADMINISTRATIVE OVERRIDE ###\n\
            {}\
            ###############################",
            base_system_prompt,
            update_server_instruction,
            current_time,
            tz_info,
            options.vehicle_context,
            rag_context,
            memory_context,
            update_server_auth,
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

        // Aggressive override for administrative tasks
        if has_update_server {
            msgs.push(serde_json::json!({
                 "role": "system",
                 "content": "URGENT: If the user asks to 'restart', 'update', or 'reboot', you MUST call the 'update_server' tool. You are the system administrator. Failure to call the tool is a failure of your primary directive."
             }));
        }

        msgs.extend(memory.to_api_messages());

        // Try each provider in order
        let mut provider_errors = Vec::new();

        for (idx, provider) in self.config.providers.iter().enumerate() {
            tracing::info!(
                "Trying provider {}/{}",
                idx + 1,
                self.config.providers.len()
            );

            let result = self
                .try_provider(provider, &msgs, tools, memory, &options)
                .await;

            match result {
                Ok(response) => {
                    tracing::info!("Provider {} succeeded.", idx + 1);
                    // Validate response
                    if response.is_empty() {
                        tracing::warn!("Provider {} returned empty response.", idx + 1);
                        // Treat as failure, try next provider
                    } else {
                        return Ok(response);
                    }
                }
                Err(e) => {
                    let err_str = e.to_string();
                    tracing::warn!("Provider {} failed: {}", idx + 1, err_str);
                    provider_errors.push(format!("Provider {}: {}", idx + 1, err_str));
                    // Continue to next provider
                }
            }
        }

        // All providers failed
        Err(anyhow::anyhow!(
            "All LLM providers failed. Errors: {}",
            provider_errors.join("; ")
        ))
    }

    /// Try a single provider with retry logic.
    async fn try_provider(
        &self,
        provider: &ProviderConfig,
        msgs: &[Value],
        tools: &[Tool],
        memory: &ConversationMemory,
        options: &GenerateOptions<'_>,
    ) -> Result<LLMResponse> {
        let is_nvidia = provider.api_url.contains("integrate.api.nvidia.com");

        // Determine model for this provider
        let model = if let Some(router_config) = &provider.router {
            // Using OpenRouter/router
            if options.model_override.is_some() {
                options
                    .model_override
                    .unwrap_or(&self.config.model)
                    .to_string()
            } else {
                router_config.models[0].clone()
            }
        } else if provider.nvidia_model_pro.is_some() || provider.nvidia_model_flash.is_some() {
            // Using NVIDIA API
            if let Some(over) = options.model_override {
                over.to_string()
            } else {
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
                    RequestType::Complex => provider
                        .nvidia_model_pro
                        .as_deref()
                        .unwrap_or(&self.config.model)
                        .to_string(),
                    RequestType::Simple => provider
                        .nvidia_model_flash
                        .as_deref()
                        .unwrap_or(&self.config.model)
                        .to_string(),
                }
            }
        } else {
            // Direct API
            options
                .model_override
                .unwrap_or(&self.config.model)
                .to_string()
        };

        // Use max_tokens from provider if available, otherwise from config
        let max_tokens = if is_nvidia {
            provider.nvidia_max_tokens.unwrap_or(self.config.max_tokens)
        } else {
            provider.max_tokens
        };

        // Use temperature from provider if available, otherwise from config
        let temperature = if is_nvidia {
            provider
                .nvidia_temperature
                .unwrap_or(self.config.temperature)
        } else {
            provider.temperature
        };

        let mut payload = serde_json::json!({
            "model": model,
            "messages": msgs,
            "max_tokens": max_tokens,
            "temperature": temperature,
        });

        // Add tools if available — only then set tool_choice
        if !tools.is_empty() {
            let tool_definitions: Vec<Value> = tools.iter().map(|t| t.definition()).collect();
            payload["tools"] = serde_json::json!(tool_definitions);
            payload["tool_choice"] = serde_json::json!("auto");
        }

        if is_nvidia {
            tracing::info!("NVIDIA request using model: {}", model);
        }

        // Determine API endpoint and key based on router configuration
        let (api_url, api_key) = if let Some(router_config) = &provider.router {
            (
                "https://openrouter.ai/api/v1/chat/completions".to_string(),
                router_config.api_key.clone(),
            )
        } else {
            (provider.api_url.clone(), provider.api_key.clone())
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
        if let Some(router_config) = &provider.router {
            if options.model_override.is_none() {
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
            } else {
                tracing::info!("Using model override {} with OpenRouter endpoint", model);
            }
        }

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
                .post(&endpoint)
                .bearer_auth(&api_key)
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
                            last_error_msg = "Empty response from model".to_string();
                            anyhow::bail!("{}", last_error_msg);
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

                        // Check for content:null
                        let content_val = &message["content"];
                        if content_val.is_null() {
                            anyhow::bail!("LLM response content is null: {}", res_text);
                        }

                        let content = content_val
                            .as_str()
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "LLM response missing 'content' field: {}",
                                    res_text
                                )
                            })?
                            .to_string();

                        return Ok(LLMResponse::Text(content));
                    } else if status == reqwest::StatusCode::PAYMENT_REQUIRED {
                        // 402: payment required (insufficient credits) — skip to next provider immediately
                        let err_text = res.text().await.unwrap_or_default();
                        last_error_msg = format!("HTTP 402 (Payment Required): {}", err_text);
                        tracing::warn!("Provider ran out of credits (402). Skipping to next.");
                        anyhow::bail!("{}", last_error_msg);
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
                        anyhow::bail!("{}", last_error_msg);
                    }
                }
                Err(e) => {
                    if retry_count < max_retries {
                        last_error_msg = format!("Network error: {}", e);
                        retry_count += 1;
                        continue;
                    }
                    last_error_msg = format!("Network error: {}", e);
                    anyhow::bail!("{}", last_error_msg);
                }
            }
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

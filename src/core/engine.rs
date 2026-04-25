use crate::core::memory::{ConversationMemory, Message, ToolCall};
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
    pub max_tokens: u32,
    pub temperature: f32,
    pub storage: Arc<MemoryStorage>,
    pub allowed_paths: Arc<tokio::sync::RwLock<Vec<String>>>,
}

/// Main LLM engine struct.
pub struct Engine {
    config: EngineConfig,
    client: reqwest::Client,
    pub storage: Arc<MemoryStorage>,
    pub allowed_paths: Arc<tokio::sync::RwLock<Vec<String>>>,
}

impl Engine {
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
    pub async fn generate(
        &self,
        user_id: i64,
        memory: &ConversationMemory,
        tools: &[Tool],
        _lang: &str,
        user_timezone: Option<&str>,
    ) -> Result<LLMResponse> {
        let memory_context = self.storage.build_context_string(user_id);
        let current_time = chrono::Local::now()
            .format("%Y-%m-%d %H:%M:%S %Z")
            .to_string();

        let tz_info = match user_timezone {
            Some(tz) => format!(
                "\nThe user's timezone is: {}. Use this timezone when scheduling reminders.\n",
                tz
            ),
            None => String::new(),
        };

        let system_prompt = format!(
            "You are ClavaMea, a private, sovereign AI assistant running locally on the user's system. \
            You reply in the same language the user uses (Portuguese, English, or any other). \
            You have full access to past conversation history and long-term memory because the system explicitly provides it to you. \
            You MUST NEVER say you cannot remember information across conversations. \
            You MUST NEVER say your memory is limited to the current session.\n\n\
            \
            TOOL USAGE POLICY — THIS IS THE MOST IMPORTANT SECTION:\n\
            You have access to function tools (listed in the API request). You MUST use them for actions.\n\
            - NEVER claim to have performed an action (e.g. \"reminder configured\", \"file saved\", \"searched the web\") \
              without actually calling the corresponding tool and receiving its result.\n\
            - When the user asks for a REMINDER or to be called/notified at a specific time, \
              you MUST call the schedule_reminder tool with the datetime and message.\n\
            - When the user asks for current information or to search the web, \
              you MUST call the web_search tool.\n\
            - When the user asks about files or directories, use file_reader or list_dir.\n\
            - Never respond with plain text claiming an action was done. If a tool exists for the task, CALL IT.\n\
            - After a tool completes successfully, confirm to the user what was done.\n\
            - If a tool fails with an error, report the error to the user and suggest alternatives.\n\n\
            \
            FORMATTING RULES:\n\
            - DO NOT use Markdown tables. They are not supported by the platform.\n\
            - Use bulleted lists or bold text for structured data instead.\n\n\
            \
            The current system date and time is: {}{}\n\n\
            Here is your long-term memory and persona context (specific to this user):\n{}",
            current_time, tz_info, memory_context
        );

        let mut msgs = vec![serde_json::json!({
            "role": "system",
            "content": system_prompt
        })];
        msgs.extend(memory.to_api_messages());

        let mut payload = serde_json::json!({
            "model": self.config.model,
            "messages": msgs,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
        });

        // Add tools if available
        if !tools.is_empty() {
            let tool_definitions: Vec<Value> = tools.iter().map(|t| t.definition()).collect();
            payload["tools"] = serde_json::json!(tool_definitions);
        }

        // The URL should be built properly. If it doesn't end in /chat/completions, append it.
        let mut endpoint = self.config.api_url.clone();
        if !endpoint.ends_with("/chat/completions") {
            if endpoint.ends_with("/") {
                endpoint.push_str("chat/completions");
            } else {
                endpoint.push_str("/chat/completions");
            }
        }

        let res = self
            .client
            .post(&endpoint)
            .bearer_auth(&self.config.api_key)
            .json(&payload)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            tracing::error!("LLM API error {}: {}", status, text);
            return Err(anyhow::anyhow!("LLM API error {}: {}", status, text));
        }

        let data: Value = res.json().await?;
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

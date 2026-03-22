//! Conversation history memory management.

use serde::{Deserialize, Serialize};

use crate::db::Interaction;

/// Role in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

/// Represents a tool call requested by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(rename = "function")]
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    pub fn user(content: String) -> Self {
        Self {
            role: Role::User,
            content: Some(content),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: String) -> Self {
        Self {
            role: Role::Assistant,
            content: Some(content),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn tool_calls(calls: Vec<ToolCall>) -> Self {
        Self {
            role: Role::Assistant,
            content: None,
            tool_calls: Some(calls),
            tool_call_id: None,
        }
    }

    pub fn tool_result(id: String, content: String) -> Self {
        Self {
            role: Role::Tool,
            content: Some(content),
            tool_calls: None,
            tool_call_id: Some(id),
        }
    }
}

/// Manages conversation history for a specific chat.
pub struct ConversationMemory {
    pub chat_id: i64,
    pub messages: Vec<Message>,
    pub max_length: usize,
}

impl ConversationMemory {
    /// Create a new empty memory for a chat.
    pub fn new(chat_id: i64, max_length: usize) -> Self {
        Self {
            chat_id,
            messages: Vec::with_capacity(max_length),
            max_length,
        }
    }

    /// Add a message to the memory.
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        if self.messages.len() > self.max_length {
            self.messages.remove(0);
        }
    }

    /// Load conversation history from database interactions.
    pub fn from_interactions(interactions: Vec<Interaction>, max_length: usize) -> Self {
        let mut messages = Vec::with_capacity(interactions.len());
        for interaction in &interactions {
            let role = match interaction.role.as_str() {
                "user" => Role::User,
                "assistant" => Role::Assistant,
                "system" => Role::System,
                _ => continue,
            };
            messages.push(Message {
                role,
                content: Some(interaction.content.clone()),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        let chat_id = interactions.first().map(|i| i.chat_id).unwrap_or(0);
        Self {
            chat_id,
            messages,
            max_length,
        }
    }

    /// Convert memory to a vector of messages for the API.
    pub fn to_api_messages(&self) -> Vec<serde_json::Value> {
        self.messages
            .iter()
            .map(|msg| serde_json::to_value(msg).unwrap_or(serde_json::json!({})))
            .collect()
    }
}

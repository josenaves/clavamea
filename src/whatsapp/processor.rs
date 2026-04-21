//! Module for processing WhatsApp messages through the LLM engine.
//!
//! This module decouples the processing logic from any particular delivery
//! mechanism (webhook or direct event).

use serde::Deserialize;
use tracing::{error, info};

use crate::bot::state::AppState;
use crate::core::{
    ConversationMemory, LLMResponse, Message as MemoryMessage, Tool, get_available_tools,
};
use crate::db::models::User;
use crate::whatsapp::sender::WhatsAppSender;

use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::Requester;

/// Incoming message payload from WhatsApp.
#[derive(Debug, Deserialize, Clone)]
pub struct WhatsAppMessagePayload {
    /// WhatsApp JID (e.g., "5511999999999@s.whatsapp.net")
    pub jid: String,
    /// Sender's phone number as numeric string (e.g., "5511999999999")
    pub phone: String,
    /// Sender's push name (display name on WhatsApp)
    pub sender_name: Option<String>,
    /// The text content of the message
    pub text: String,
}

/// Context for processing a WhatsApp message.
#[derive(Clone)]
pub struct WhatsAppProcessor {
    pub app_state: AppState,
    pub sender: WhatsAppSender,
}

impl WhatsAppProcessor {
    pub fn new(app_state: AppState, sender: WhatsAppSender) -> Self {
        Self { app_state, sender }
    }

    /// Process an incoming WhatsApp message.
    pub async fn handle_message(&self, payload: WhatsAppMessagePayload) {
        info!(
            "Processing WhatsApp message from {} ({}): {}",
            payload.phone,
            payload.sender_name.as_deref().unwrap_or("unknown"),
            payload.text
        );

        // Use the phone number (digits only) as user ID.
        // WhatsApp phone numbers are globally unique, so we parse them as i64.
        let user_id: i64 = match payload.phone.parse() {
            Ok(id) => id,
            Err(e) => {
                error!(
                    "Failed to parse phone number '{}' as i64: {}",
                    payload.phone, e
                );
                return;
            }
        };

        // ── Access Control (same as Telegram) ──────────────────────────────
        let user_record: Option<User> = crate::db::queries::get_user(&self.app_state.db_pool, user_id)
            .await
            .unwrap_or(None);

        let is_authorized = match user_record {
            Some(ref u) => u.authorized,
            None => {
                // Register new user as pending
                let username = payload.sender_name.as_deref();
                let _ = crate::db::queries::create_user_pending(
                    &self.app_state.db_pool,
                    user_id,
                    username,
                )
                .await;

                // Notify the owner via Telegram about the new pending WhatsApp user
                let owner_chat_id = teloxide::types::ChatId(self.app_state.owner_id);
                let admin_msg = format!(
                    "🔔 **Novo usuário WhatsApp pendente!**\n\nTelefone: `{}`\nNome: {}\n\nUse `/approve {} <papel> <nome>` para liberar.",
                    payload.phone,
                    payload.sender_name.as_deref().unwrap_or("não informado"),
                    user_id
                );

                let renderer = crate::core::renderer::TelegramMarkdownV2Renderer::new();
                let rendered_admin = crate::core::Renderer::render(&renderer, &admin_msg);
                let _ = self.app_state
                    .bot
                    .send_message(owner_chat_id, rendered_admin)
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                    .await;

                false
            }
        };

        let is_owner = user_id == self.app_state.owner_id;
        let is_admin = is_owner
            || user_record
                .as_ref()
                .map(|u: &User| u.is_admin())
                .unwrap_or(false);

        if !is_authorized && !is_admin {
            // Send access denied via WhatsApp
            let _ = self.sender
                .send_message(
                    &payload.jid,
                    "🚫 Acesso Restrito\n\nDesculpe, você não tem permissão para usar este bot. Sua solicitação de acesso foi enviada para o administrador.",
                )
                .await;
            return;
        }

        // ── Process message through the LLM Engine ─────────────────────────
        let jid = payload.jid.clone();
        let text = payload.text.clone();
        let this = self.clone();

        // Spawn processing in background
        tokio::spawn(async move {
            if let Err(e) = this.process_core(user_id, &jid, &text).await {
                error!("Error processing WhatsApp message: {}", e);
                let _ = this.sender
                    .send_message(&jid, "Desculpe, ocorreu um erro ao processar sua mensagem.")
                    .await;
            }
        });
    }

    /// Internal core processing loop.
    async fn process_core(
        &self,
        user_id: i64,
        jid: &str,
        text: &str,
    ) -> anyhow::Result<()> {
        let chat_id = user_id; // Use phone number as chat_id for DB interactions
        let lang = "en";

        // Insert user interaction into DB
        let user_interaction = crate::db::models::NewInteraction::user(chat_id, text.to_string(), lang);
        if let Err(e) =
            crate::db::queries::insert_interaction(&self.app_state.db_pool, &user_interaction).await
        {
            error!("Failed to save WhatsApp user interaction: {}", e);
        }

        // Load conversation history
        let history = crate::db::queries::get_recent_interactions(
            &self.app_state.db_pool,
            chat_id,
            self.app_state.max_conversation_length as u32,
        )
        .await
        .unwrap_or_else(|_| Vec::new());

        let mut memory =
            ConversationMemory::from_interactions(history, self.app_state.max_conversation_length);
        memory.add_message(MemoryMessage::user(text.to_string()));

        // Available tools (same set as Telegram)
        let tools = get_available_tools(3);
        let mut turn = 0;
        let max_turns = 20;

        // Create a dummy Bot and ChatId for tool execution compatibility
        let dummy_chat_id = teloxide::types::ChatId(chat_id);

        loop {
            if turn >= max_turns {
                let _ = self.sender
                    .send_message(
                        jid,
                        "Atingi o limite máximo de raciocínio para esta conversa.",
                    )
                    .await;
                break;
            }

            match self.app_state
                .engine
                .generate(user_id, &memory, &tools, lang)
                .await
            {
                Ok(LLMResponse::Text(content)) => {
                    // Save assistant interaction
                    let assistant_interaction =
                        crate::db::models::NewInteraction::assistant(chat_id, content.clone(), lang);
                    if let Err(e) = crate::db::queries::insert_interaction(
                        &self.app_state.db_pool,
                        &assistant_interaction,
                    )
                    .await
                    {
                        error!("Failed to save WhatsApp assistant interaction: {}", e);
                    }

                    // Send plain text response via WhatsApp
                    let _ = self.sender.send_message(jid, &content).await;
                    break;
                }
                Ok(LLMResponse::ToolCalls(tool_calls)) => {
                    // LLM requested tool execution
                    memory.add_message(MemoryMessage::tool_calls(tool_calls.clone()));

                    for tool_call in tool_calls {
                        let tool_name = tool_call.function.name.as_str();
                        let args: serde_json::Value =
                            match serde_json::from_str(&tool_call.function.arguments) {
                                Ok(v) => v,
                                Err(e) => {
                                    memory.add_message(MemoryMessage::tool_result(
                                        tool_call.id.clone(),
                                        format!("Invalid arguments: {}", e),
                                    ));
                                    continue;
                                }
                            };

                        info!("WhatsApp LLM requested tool: {}", tool_name);

                        let tool_option = Tool::from_name(tool_name);

                        if let Some(tool) = tool_option {
                            match tool
                                .execute(
                                    &self.app_state.bot,
                                    dummy_chat_id,
                                    user_id,
                                    &args,
                                    self.app_state.engine.storage.clone(),
                                    self.app_state.rag.clone(),
                                    self.app_state.wasm.clone(),
                                    self.app_state.engine.allowed_paths.clone(),
                                    &self.app_state.db_pool,
                                )
                                .await
                            {
                                Ok(result) => {
                                    memory.add_message(MemoryMessage::tool_result(
                                        tool_call.id.clone(),
                                        result,
                                    ));
                                }
                                Err(e) => {
                                    error!("WhatsApp tool execution error: {}", e);
                                    memory.add_message(MemoryMessage::tool_result(
                                        tool_call.id.clone(),
                                        format!("Error: {}", e),
                                    ));
                                }
                            }
                        } else {
                            memory.add_message(MemoryMessage::tool_result(
                                tool_call.id.clone(),
                                format!("Unknown tool: {}", tool_name),
                            ));
                        }
                    }
                    turn += 1;
                }
                Err(e) => {
                    error!("WhatsApp engine error: {}", e);
                    let _ = self.sender
                        .send_message(jid, "Desculpe, ocorreu um erro ao gerar a resposta.")
                        .await;
                    break;
                }
            }
        }

        Ok(())
    }
}

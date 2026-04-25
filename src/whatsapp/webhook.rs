//! Axum webhook handler for receiving WhatsApp messages from the bridge.
//!
//! This module provides the HTTP endpoints that receive incoming messages
//! from the whatsapp-bridge service and processes them through the same
//! LLM Engine used by Telegram.

use axum::extract::State;
use axum::http::StatusCode;
use axum::{Json, Router, routing::post};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::bot::state::AppState;
use crate::core::{
    ConversationMemory, LLMResponse, Message as MemoryMessage, Tool, get_available_tools,
};
use crate::db::models::User;
use crate::whatsapp::sender::WhatsAppSender;

use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::Requester;

/// Incoming message payload from the WhatsApp bridge.
#[derive(Debug, Deserialize)]
pub struct WebhookPayload {
    /// WhatsApp JID (e.g., "5511999999999@s.whatsapp.net")
    pub jid: String,
    /// Sender's phone number as numeric string (e.g., "5511999999999")
    pub phone: String,
    /// Sender's push name (display name on WhatsApp)
    pub sender_name: Option<String>,
    /// The text content of the message
    pub text: String,
}

/// Response sent back to the bridge after processing.
#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub ok: bool,
    pub message: Option<String>,
}

/// Shared state for the WhatsApp webhook server.
#[derive(Clone)]
pub struct WhatsAppWebhookState {
    /// The shared ClavaMea application state.
    pub app_state: AppState,
    /// Client for sending messages back through the bridge.
    pub sender: WhatsAppSender,
}

/// Create the Axum router for WhatsApp webhook endpoints.
pub fn create_router(state: WhatsAppWebhookState) -> Router {
    Router::new()
        .route("/wa/webhook", post(handle_webhook))
        .route("/wa/health", axum::routing::get(handle_health))
        .with_state(state)
}

/// Health check endpoint.
async fn handle_health() -> &'static str {
    "OK"
}

/// Main webhook handler — processes an incoming WhatsApp message.
async fn handle_webhook(
    State(state): State<WhatsAppWebhookState>,
    Json(payload): Json<WebhookPayload>,
) -> (StatusCode, Json<WebhookResponse>) {
    info!(
        "WhatsApp webhook received message from {} ({}): {}",
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
            return (
                StatusCode::BAD_REQUEST,
                Json(WebhookResponse {
                    ok: false,
                    message: Some("Invalid phone number".to_string()),
                }),
            );
        }
    };

    // ── Access Control (same as Telegram) ──────────────────────────────
    let user_record: Option<User> = crate::db::queries::get_user(&state.app_state.db_pool, user_id)
        .await
        .unwrap_or(None);

    let is_authorized = match user_record {
        Some(ref u) => u.authorized,
        None => {
            // Register new user as pending
            let username = payload.sender_name.as_deref();
            let _ = crate::db::queries::create_user_pending(
                &state.app_state.db_pool,
                user_id,
                username,
            )
            .await;

            // Notify the owner via Telegram about the new pending WhatsApp user
            let owner_chat_id = teloxide::types::ChatId(state.app_state.owner_id);
            let admin_msg = format!(
                "🔔 **Novo usuário WhatsApp pendente!**\n\nTelefone: `{}`\nNome: {}\n\nUse `/approve {} <papel> <nome>` para liberar.",
                payload.phone,
                payload.sender_name.as_deref().unwrap_or("não informado"),
                user_id
            );

            let renderer = crate::core::renderer::TelegramMarkdownV2Renderer::new();
            let rendered_admin = crate::core::Renderer::render(&renderer, &admin_msg);
            let _ = state
                .app_state
                .bot
                .send_message(owner_chat_id, rendered_admin)
                .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                .await;

            false
        }
    };

    let is_owner = user_id == state.app_state.owner_id;
    let is_admin = is_owner
        || user_record
            .as_ref()
            .map(|u: &User| u.is_admin())
            .unwrap_or(false);

    if !is_authorized && !is_admin {
        // Send access denied via WhatsApp
        let _ = state
            .sender
            .send_message(
                &payload.jid,
                "🚫 Acesso Restrito\n\nDesculpe, você não tem permissão para usar este bot. Sua solicitação de acesso foi enviada para o administrador.",
            )
            .await;

        return (
            StatusCode::FORBIDDEN,
            Json(WebhookResponse {
                ok: false,
                message: Some("User not authorized".to_string()),
            }),
        );
    }

    // ── Process message through the LLM Engine ─────────────────────────
    let jid = payload.jid.clone();
    let sender = state.sender.clone();

    // Spawn processing in background so the webhook returns immediately
    tokio::spawn(async move {
        if let Err(e) = process_whatsapp_message(state, user_id, &jid, &payload.text).await {
            error!("Error processing WhatsApp message: {}", e);
            let _ = sender
                .send_message(&jid, "Desculpe, ocorreu um erro ao processar sua mensagem.")
                .await;
        }
    });

    (
        StatusCode::OK,
        Json(WebhookResponse {
            ok: true,
            message: Some("Processing".to_string()),
        }),
    )
}

/// Process a WhatsApp message through the LLM engine and send the response back.
async fn process_whatsapp_message(
    state: WhatsAppWebhookState,
    user_id: i64,
    jid: &str,
    text: &str,
) -> anyhow::Result<()> {
    let chat_id = user_id; // Use phone number as chat_id for DB interactions
    let lang = "en";

    // Insert user interaction into DB
    let user_interaction = crate::db::models::NewInteraction::user(chat_id, text.to_string(), lang);
    if let Err(e) =
        crate::db::queries::insert_interaction(&state.app_state.db_pool, &user_interaction).await
    {
        error!("Failed to save WhatsApp user interaction: {}", e);
    }

    // Load conversation history
    let history = crate::db::queries::get_recent_interactions(
        &state.app_state.db_pool,
        chat_id,
        state.app_state.max_conversation_length as u32,
    )
    .await
    .unwrap_or_else(|_| Vec::new());

    let mut memory =
        ConversationMemory::from_interactions(history, state.app_state.max_conversation_length);
    memory.add_message(MemoryMessage::user(text.to_string()));

    // Available tools (same set as Telegram)
    let tools = get_available_tools(3);
    let mut turn = 0;
    let max_turns = 20;

    // Look up user timezone if configured
    let user_tz = crate::db::queries::get_user(&state.app_state.db_pool, user_id)
        .await
        .ok()
        .flatten()
        .and_then(|u| u.timezone);

    // Create a dummy Bot and ChatId for tool execution compatibility
    // Tools that require sending Telegram files will gracefully fail for WhatsApp
    let dummy_chat_id = teloxide::types::ChatId(chat_id);

    loop {
        if turn >= max_turns {
            let _ = state
                .sender
                .send_message(
                    jid,
                    "Atingi o limite máximo de raciocínio para esta conversa.",
                )
                .await;
            break;
        }

        match state
            .app_state
            .engine
            .generate(user_id, &memory, &tools, lang, user_tz.as_deref())
            .await
        {
            Ok(LLMResponse::Text(content)) => {
                // Save assistant interaction
                let assistant_interaction =
                    crate::db::models::NewInteraction::assistant(chat_id, content.clone(), lang);
                if let Err(e) = crate::db::queries::insert_interaction(
                    &state.app_state.db_pool,
                    &assistant_interaction,
                )
                .await
                {
                    error!("Failed to save WhatsApp assistant interaction: {}", e);
                }

                // Send plain text response via WhatsApp (no Telegram Markdown formatting)
                let _ = state.sender.send_message(jid, &content).await;
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
                                &state.app_state.bot,
                                dummy_chat_id,
                                user_id,
                                &args,
                                state.app_state.engine.storage.clone(),
                                state.app_state.rag.clone(),
                                state.app_state.wasm.clone(),
                                state.app_state.engine.allowed_paths.clone(),
                                &state.app_state.db_pool,
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
                let _ = state
                    .sender
                    .send_message(jid, "Desculpe, ocorreu um erro ao gerar a resposta.")
                    .await;
                break;
            }
        }
    }

    Ok(())
}

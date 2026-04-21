//! WhatsApp Bridge for ClavaMea.
//!
//! This binary connects to WhatsApp Web via oxidezap/whatsapp-rust and acts as
//! a bridge between WhatsApp and the ClavaMea AI assistant. It:
//!
//! 1. Authenticates via QR code (first run) or persisted session (subsequent runs)
//! 2. Forwards incoming WhatsApp messages to ClavaMea via HTTP webhook
//! 3. Exposes an HTTP endpoint to receive replies from ClavaMea and send them back

use std::env;
use std::sync::Arc;

use axum::{Json, Router, extract::State, routing::post};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use whatsapp_rust::bot::{Bot, MessageContext};
use whatsapp_rust::store::SqliteStore;
use whatsapp_rust::TokioRuntime;
use whatsapp_rust_tokio_transport::TokioWebSocketTransportFactory;
use whatsapp_rust_ureq_http_client::UreqHttpClient;

// Internal wacore/waproto types re-exported or used via crates
use whatsapp_rust::types::events::Event;
use whatsapp_rust::proto_helpers::MessageExt;
use whatsapp_rust::waproto::whatsapp as wa;

/// Message payload sent to ClavaMea webhook.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IncomingMessage {
    /// WhatsApp JID (e.g., "5511999999999@s.whatsapp.net")
    pub jid: String,
    /// Sender's phone number as numeric string (e.g., "5511999999999")
    pub phone: String,
    /// Sender's push name (display name on WhatsApp)
    pub sender_name: Option<String>,
    /// The text content of the message
    pub text: String,
}

/// Message payload received from ClavaMea to send back via WhatsApp.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OutgoingMessage {
    /// WhatsApp JID to send the message to
    pub jid: String,
    /// The text content to send
    pub text: String,
}

/// Response for the /send endpoint.
#[derive(Debug, Serialize)]
pub struct SendResponse {
    pub ok: bool,
    pub error: Option<String>,
}

/// Shared state for the HTTP server.
struct BridgeState {
    /// WhatsApp client for sending messages.
    client: RwLock<Option<Arc<whatsapp_rust::client::Client>>>,
}

/// Handler for POST /send — receives messages from ClavaMea and sends them via WhatsApp.
async fn handle_send(
    State(state): State<Arc<BridgeState>>,
    Json(msg): Json<OutgoingMessage>,
) -> Json<SendResponse> {
    info!("Sending message to WhatsApp JID: {}", msg.jid);

    let client_guard = state.client.read().await;
    match client_guard.as_ref() {
        Some(client) => {
            let jid_res = msg.jid.parse();
            match jid_res {
                Ok(jid) => {
                    let wa_msg = wa::Message {
                        conversation: Some(msg.text),
                        ..Default::default()
                    };
                    
                    match client.send_message(jid, wa_msg).await {
                        Ok(_) => {
                            info!("Message sent successfully to {}", msg.jid);
                            Json(SendResponse { ok: true, error: None })
                        }
                        Err(e) => {
                            error!("Failed to send message to {}: {}", msg.jid, e);
                            Json(SendResponse { ok: false, error: Some(format!("Send failed: {}", e)) })
                        }
                    }
                },
                Err(e) => {
                    error!("Invalid JID {}: {}", msg.jid, e);
                    Json(SendResponse { ok: false, error: Some(format!("Invalid JID: {}", e)) })
                }
            }
        }
        None => {
            warn!("WhatsApp client not yet connected");
            Json(SendResponse {
                ok: false,
                error: Some("Client not connected".to_string()),
            })
        }
    }
}

/// Health check endpoint.
async fn handle_health() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env from parent directory (shared with ClavaMea)
    dotenv::from_filename("../.env").ok();
    dotenv::dotenv().ok();

    tracing_subscriber::fmt::init();

    info!("Starting WhatsApp Bridge for ClavaMea...");

    // Configuration
    let clavamea_webhook_url = env::var("CLAVAMEA_WEBHOOK_URL")
        .unwrap_or_else(|_| "http://localhost:8081/wa/webhook".to_string());
    let bridge_port: u16 = env::var("WHATSAPP_BRIDGE_PORT")
        .unwrap_or_else(|_| "8082".to_string())
        .parse()
        .expect("WHATSAPP_BRIDGE_PORT must be a valid port number");
    let wa_db_path = env::var("WHATSAPP_DB_PATH")
        .unwrap_or_else(|_| "data/whatsapp.db".to_string());

    info!("ClavaMea webhook URL: {}", clavamea_webhook_url);
    info!("Bridge HTTP port: {}", bridge_port);
    info!("WhatsApp DB path: {}", wa_db_path);

    // Ensure data directory exists
    if let Some(parent) = std::path::Path::new(&wa_db_path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // Initialize WhatsApp store
    let backend = Arc::new(SqliteStore::new(&wa_db_path).await?);

    // Shared state for the HTTP server
    let bridge_state = Arc::new(BridgeState {
        client: RwLock::new(None),
    });

    // Clone for the event handler
    let webhook_url = clavamea_webhook_url.clone();
    let http_client = reqwest::Client::new();

    // Build the WhatsApp bot
    let bridge_state_for_bot = bridge_state.clone();
    let mut bot = Bot::builder()
        .with_backend(backend)
        .with_transport_factory(TokioWebSocketTransportFactory::new())
        .with_http_client(UreqHttpClient::new())
        .with_runtime(TokioRuntime)
        .on_event(move |event, client| {
            let webhook_url = webhook_url.clone();
            let http_client = http_client.clone();
            let bridge_state = bridge_state_for_bot.clone();

            async move {
                match event {
                    Event::PairingQrCode { code, .. } => {
                        println!("\n╔══════════════════════════════════════════╗");
                        println!("║   SCAN THIS QR CODE WITH WHATSAPP       ║");
                        println!("╠══════════════════════════════════════════╣");
                        println!("║                                          ║");
                        println!("  {}", code);
                        println!("║                                          ║");
                        println!("╚══════════════════════════════════════════╝\n");
                        info!("QR Code displayed. Waiting for scan...");
                    }
                    Event::Connected(_) => {
                        info!("✅ WhatsApp connected successfully!");
                        // Store the client reference for sending messages
                        let mut client_guard = bridge_state.client.write().await;
                        *client_guard = Some(client);
                    }
                    Event::Message(msg, info) => {
                        let ctx = MessageContext {
                            message: msg,
                            info,
                            client,
                        };

                        let sender_jid = ctx.info.source.sender.to_string();

                        // Extract phone number from JID (format: "5511999999999@s.whatsapp.net")
                        let phone = sender_jid
                            .split('@')
                            .next()
                            .unwrap_or(&sender_jid)
                            .to_string();

                        // Extract text content from the message
                        let text = match ctx.message.text_content() {
                            Some(t) => t.to_string(),
                            None => {
                                info!("Ignoring non-text message from {}", sender_jid);
                                return;
                            }
                        };

                        info!("Received message from {}: {}", phone, text);

                        let payload = IncomingMessage {
                            jid: sender_jid.clone(),
                            phone,
                            sender_name: Some(ctx.info.push_name.clone()),
                            text,
                        };

                        // Forward to ClavaMea webhook
                        match http_client
                            .post(&webhook_url)
                            .json(&payload)
                            .send()
                            .await
                        {
                            Ok(resp) => {
                                if resp.status().is_success() {
                                    info!("Message forwarded to ClavaMea successfully");
                                } else {
                                    error!(
                                        "ClavaMea webhook returned error: {}",
                                        resp.status()
                                    );
                                }
                            }
                            Err(e) => {
                                error!("Failed to forward message to ClavaMea: {}", e);
                            }
                        }
                    }
                    Event::LoggedOut(_) => {
                        warn!("WhatsApp logged out");
                        let mut client_guard = bridge_state.client.write().await;
                        *client_guard = None;
                    }
                    _ => {}
                }
            }
        })
        .build()
        .await?;

    // Start the HTTP server for receiving outgoing messages from ClavaMea
    let app = Router::new()
        .route("/send", post(handle_send))
        .route("/health", axum::routing::get(handle_health))
        .with_state(bridge_state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", bridge_port)).await?;
    info!("Bridge HTTP server listening on port {}", bridge_port);

    // Run both the WhatsApp bot and the HTTP server concurrently
    let bot_handle = bot.run().await?;

    tokio::select! {
        result = axum::serve(listener, app) => {
            if let Err(e) = result {
                error!("HTTP server error: {}", e);
            }
        }
        result = bot_handle => {
            if let Err(e) = result {
                error!("WhatsApp bot error: {}", e);
            }
        }
    }

    Ok(())
}

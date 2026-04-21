//! Manager for the WhatsApp background service.
//!
//! This module initializes the oxidezap/whatsapp-rust bot and handles
//! its event loop, forwarding messages to the processor.

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use whatsapp_rust::bot::{Bot, MessageContext};
use whatsapp_rust::proto_helpers::MessageExt;
use whatsapp_rust::types::events::Event;
use whatsapp_rust::TokioRuntime;
use whatsapp_rust_tokio_transport::TokioWebSocketTransportFactory;
use whatsapp_rust_ureq_http_client::UreqHttpClient;

use crate::whatsapp::processor::{WhatsAppMessagePayload, WhatsAppProcessor};
use crate::whatsapp::store::SqlxWhatsAppStore;

/// Manages the WhatsApp bot connection and events.
pub struct WhatsAppManager {
    /// Database pool for persistence.
    pub db_pool: sqlx::Pool<sqlx::Sqlite>,
    /// The shared client for sending messages.
    pub client: Arc<RwLock<Option<Arc<whatsapp_rust::client::Client>>>>,
    /// The processor for incoming messages.
    pub processor: WhatsAppProcessor,
}

impl WhatsAppManager {
    pub fn new(db_pool: sqlx::Pool<sqlx::Sqlite>, processor: WhatsAppProcessor) -> Self {
        Self {
            db_pool,
            client: Arc::new(RwLock::new(None)),
            processor,
        }
    }

    /// Starts the WhatsApp bot event loop.
    pub async fn run(&self) -> anyhow::Result<()> {
        info!("Initializing integrated WhatsApp service...");

        // Initialize our custom SQLx-based store
        let wa_store = SqlxWhatsAppStore::new(self.db_pool.clone());
        wa_store.init().await?;

        let backend = Arc::new(wa_store);

        // Clone for the event handler
        let client_state = self.client.clone();
        let processor = self.processor.clone();

        // Build the WhatsApp bot
        let mut bot = Bot::builder()
            .with_backend(backend)
            .with_transport_factory(TokioWebSocketTransportFactory::new())
            .with_http_client(UreqHttpClient::new())
            .with_runtime(TokioRuntime)
            .on_event(move |event, client| {
                let client_state = client_state.clone();
                let processor = processor.clone();

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
                            info!("WhatsApp QR Code displayed (Integrated). Waiting for scan...");
                        }
                        Event::Connected(_) => {
                            info!("✅ WhatsApp (Integrated) connected successfully!");
                            let mut client_guard = client_state.write().await;
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
                                    info!(
                                        "Ignoring non-text message from WhatsApp: {}",
                                        sender_jid
                                    );
                                    return;
                                }
                            };

                            info!("Received WhatsApp message from {}: {}", phone, text);

                            let payload = WhatsAppMessagePayload {
                                jid: sender_jid,
                                phone,
                                sender_name: Some(ctx.info.push_name.clone()),
                                text,
                            };

                            // Direct call to processor
                            processor.handle_message(payload).await;
                        }
                        Event::LoggedOut(_) => {
                            warn!("WhatsApp logged out (Integrated)");
                            let mut client_guard = client_state.write().await;
                            *client_guard = None;
                        }
                        _ => {}
                    }
                }
            })
            .build()
            .await?;

        info!("Starting integrated WhatsApp bot event loop...");
        let bot_handle = bot.run().await?;

        // Wait for the bot handle (it runs indefinitely)
        bot_handle.await?;

        Ok(())
    }
}

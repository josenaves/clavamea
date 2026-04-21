//! Client for sending messages directly via the local WhatsApp service.

use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;
use tracing::{error, info, warn};

// Import necessary waproto types from whatsapp-rust
use whatsapp_rust::waproto::whatsapp as wa;

/// Client for sending messages via the local WhatsApp connection.
#[derive(Clone)]
pub struct WhatsAppSender {
    /// Shared reference to the WhatsApp client.
    client_state: Arc<RwLock<Option<Arc<whatsapp_rust::client::Client>>>>,
}

impl WhatsAppSender {
    /// Create a new sender with the shared client state.
    pub fn new(client_state: Arc<RwLock<Option<Arc<whatsapp_rust::client::Client>>>>) -> Self {
        Self { client_state }
    }

    /// Send a text message to a WhatsApp JID directly.
    pub async fn send_message(&self, jid_str: &str, text: &str) -> Result<()> {
        let client_guard = self.client_state.read().await;
        
        match client_guard.as_ref() {
            Some(client) => {
                let jid = jid_str.parse().map_err(|e| anyhow::anyhow!("Invalid JID {}: {}", jid_str, e))?;
                
                let wa_msg = wa::Message {
                    conversation: Some(text.to_string()),
                    ..Default::default()
                };
                
                info!("Sending direct WhatsApp message to JID: {}", jid_str);
                
                match client.send_message(jid, wa_msg).await {
                    Ok(_) => {
                        info!("WhatsApp message delivered successfully");
                        Ok(())
                    }
                    Err(e) => {
                        error!("Failed to send WhatsApp message to {}: {}", jid_str, e);
                        Err(anyhow::anyhow!("Send failed: {}", e))
                    }
                }
            }
            None => {
                warn!("WhatsApp client not yet connected. Cannot send message.");
                Err(anyhow::anyhow!("WhatsApp client not connected"))
            }
        }
    }

    /// Health check (check if client is connected).
    pub async fn health_check(&self) -> bool {
        let client_guard = self.client_state.read().await;
        client_guard.is_some()
    }
}

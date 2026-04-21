//! HTTP client for sending messages back to the WhatsApp bridge.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

/// Payload for sending a message via the WhatsApp bridge.
#[derive(Debug, Serialize)]
pub struct SendPayload {
    pub jid: String,
    pub text: String,
}

/// Response from the WhatsApp bridge /send endpoint.
#[derive(Debug, Deserialize)]
pub struct SendResponse {
    pub ok: bool,
    pub error: Option<String>,
}

/// Client for communicating with the WhatsApp bridge.
#[derive(Clone)]
pub struct WhatsAppSender {
    http_client: reqwest::Client,
    bridge_url: String,
}

impl WhatsAppSender {
    /// Create a new sender pointing to the given bridge base URL.
    pub fn new(bridge_url: &str) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            bridge_url: bridge_url.trim_end_matches('/').to_string(),
        }
    }

    /// Send a text message to a WhatsApp JID via the bridge.
    pub async fn send_message(&self, jid: &str, text: &str) -> Result<()> {
        let url = format!("{}/send", self.bridge_url);
        let payload = SendPayload {
            jid: jid.to_string(),
            text: text.to_string(),
        };

        info!("Sending message to WhatsApp bridge for JID: {}", jid);

        let resp = self.http_client.post(&url).json(&payload).send().await?;

        if resp.status().is_success() {
            let body: SendResponse = resp.json().await?;
            if body.ok {
                info!("Message delivered to WhatsApp bridge successfully");
                Ok(())
            } else {
                let err_msg = body.error.unwrap_or_else(|| "Unknown error".to_string());
                error!("WhatsApp bridge returned error: {}", err_msg);
                Err(anyhow::anyhow!("Bridge error: {}", err_msg))
            }
        } else {
            let status = resp.status();
            error!("WhatsApp bridge HTTP error: {}", status);
            Err(anyhow::anyhow!("Bridge HTTP error: {}", status))
        }
    }

    /// Check if the bridge is healthy.
    pub async fn health_check(&self) -> bool {
        let url = format!("{}/health", self.bridge_url);
        match self.http_client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}

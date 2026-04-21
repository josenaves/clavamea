//! WhatsApp integration module.
//!
//! Provides direct connection to WhatsApp Web via oxidezap/whatsapp-rust,
//! local message processing, and internal sender logic.

pub mod manager;
pub mod processor;
pub mod sender;
pub mod store;

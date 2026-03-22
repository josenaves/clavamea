//! Shared application state.

use crate::core::{Engine, RagManager};
use crate::db::Pool;
use crate::i18n::BundleManager;
use std::sync::Arc;

/// Global application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    /// Database connection pool.
    pub db_pool: Pool,
    /// LLM engine for generating responses.
    pub engine: Arc<Engine>,
    /// Internationalization bundle manager.
    pub i18n: Arc<BundleManager>,
    /// RAG manager for document search.
    pub rag: Arc<RagManager>,
    /// Wasm runtime for code execution.
    pub wasm: Arc<crate::core::wasm::WasmRuntime>,
    /// Owner's Telegram user ID.
    pub owner_id: i64,
    /// Maximum conversation history length.
    pub max_conversation_length: usize,
    /// Telegram bot instance for proactive messaging.
    pub bot: teloxide::Bot,
}

impl AppState {
    /// Create a new AppState with the given components.
    pub fn new(
        db_pool: Pool,
        engine: Arc<Engine>,
        i18n: Arc<BundleManager>,
        rag: Arc<RagManager>,
        wasm: Arc<crate::core::wasm::WasmRuntime>,
        owner_id: i64,
        max_conversation_length: usize,
        bot: teloxide::Bot,
    ) -> Self {
        Self {
            db_pool,
            engine,
            i18n,
            rag,
            wasm,
            owner_id,
            max_conversation_length,
            bot,
        }
    }

    /// Check if a user ID matches the owner.
    pub fn is_owner(&self, user_id: i64) -> bool {
        user_id == self.owner_id
    }
}

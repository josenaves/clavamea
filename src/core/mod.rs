//! LLM orchestration, prompt building, and tool calling logic.
//!
//! This module handles the core AI functionality:
//! - Communication with OpenAI-compatible APIs (DeepSeek)
//! - Conversation history management
//! - Prompt engineering with language context
//! - Tool calling infrastructure for future expansion

#![allow(unused_imports)]

pub mod engine;
pub mod memory;
pub mod prompt;
pub mod storage;
pub mod tools;
pub mod wasm;
pub mod rag;
pub mod renderer;
pub mod genetics;

pub use engine::*;
pub use memory::*;
pub use prompt::*;
pub use storage::*;
pub use tools::*;
pub use rag::*;
pub use renderer::*;


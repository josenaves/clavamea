//! Telegram bot handlers and message routing.
//!
//! This module contains all Telegram-specific logic, including:
//! - Message filtering by owner ID
//! - Command parsing and routing
//! - Response formatting with MarkdownV2
//! - State management for the dispatcher

#![allow(unused_imports)]

pub mod handlers;
pub mod router;
pub mod state;

pub use handlers::*;
pub use router::*;
pub use state::*;
pub mod scheduler;

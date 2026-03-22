//! Translation loading and language detection logic.
//!
//! This module implements internationalization using Project Fluent:
//! - Loading `.ftl` files from the `locales/` directory
//! - Detecting user language from Telegram metadata
//! - Providing localized strings for bot responses
//! - Fallback chain (en → pt-BR → en)

#![allow(unused_imports)]

pub mod bundle;
pub mod detection;
pub mod loader;

pub use bundle::*;
pub use detection::*;
pub use loader::*;

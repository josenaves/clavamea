//! Fluent bundle management and localization.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use fluent::{FluentArgs, FluentResource};
use fluent_bundle::concurrent::FluentBundle;

use crate::i18n::loader;

type ConcurrentBundle = FluentBundle<FluentResource>;

/// Manages Fluent bundles for multiple languages.
pub struct BundleManager {
    bundles: HashMap<String, Arc<ConcurrentBundle>>,
    fallback_lang: String,
}

impl BundleManager {
    /// Create a new bundle manager loading bundles from the given directory.
    pub fn new(locales_dir: &Path, fallback_lang: &str) -> Result<Self> {
        let mut bundles = HashMap::new();

        // Load available languages
        let languages = loader::available_languages(locales_dir)?;
        for lang in languages {
            match loader::load_bundle(locales_dir, &lang) {
                Ok(bundle) => {
                    bundles.insert(lang.clone(), Arc::new(bundle));
                }
                Err(e) => {
                    tracing::warn!("Failed to load bundle for {}: {}", lang, e);
                }
            }
        }

        Ok(Self {
            bundles,
            fallback_lang: fallback_lang.to_string(),
        })
    }

    /// Get a message from the appropriate bundle.
    pub fn get_message(&self, lang: &str, message_id: &str, args: Option<&FluentArgs>) -> String {
        let bundle = self
            .bundles
            .get(lang)
            .or_else(|| self.bundles.get(&self.fallback_lang))
            .cloned();

        if let Some(bundle) = bundle {
            if let Some(message) = bundle.get_message(message_id) {
                if let Some(pattern) = message.value() {
                    let mut errors = Vec::new();
                    let result = bundle.format_pattern(pattern, args, &mut errors);
                    return result.to_string();
                }
            }
        }

        // Fallback to message ID if translation not found
        message_id.to_string()
    }

    /// Check if a language is supported.
    pub fn supports_language(&self, lang: &str) -> bool {
        self.bundles.contains_key(lang)
    }
}

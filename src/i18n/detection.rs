//! Language detection from Telegram metadata.

use teloxide::types::User;

/// Detect the user's preferred language from Telegram metadata.
///
/// Falls back to English if no language is specified or if the language
/// is not supported.
pub fn detect_language(user: &User) -> String {
    // Try to get language_code from Telegram user object
    let lang = user
        .language_code
        .as_ref()
        .map(|lc| lc.to_lowercase())
        .unwrap_or_default();

    // Normalize language codes
    match lang.as_str() {
        "pt" | "pt-br" | "pt_br" => "pt-BR".to_string(),
        "en" | "en-us" | "en_gb" => "en".to_string(),
        // Add more languages as needed
        _ => "en".to_string(), // Fallback to English
    }
}

/// Validate if a language code is supported by the application.
pub fn is_supported_language(lang: &str) -> bool {
    matches!(lang, "en" | "pt-BR")
}

/// Get the fallback language chain for a given language.
///
/// Returns a vector of language codes to try in order of preference.
pub fn fallback_chain(lang: &str) -> Vec<&'static str> {
    match lang {
        "pt-BR" => vec!["pt-BR", "en"],
        "en" => vec!["en"],
        _ => vec!["en"],
    }
}

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestType {
    Complex,
    Simple,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RouterConfig {
    pub api_key: String,
    pub models: Vec<String>,
    pub timeout: u64,
    #[serde(skip)]
    pub blacklist: Arc<Mutex<HashSet<String>>>,
}

impl RouterConfig {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("OPENROUTER_API_KEY").ok()?;
        let models_str = std::env::var("OPENROUTER_MODELS")
            .unwrap_or_else(|_| "google/gemini-2.0-flash,openai/gpt-4o-mini".to_string());
        let mut models: Vec<String> = models_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        // Shuffle models for randomization
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .hash(&mut hasher);
        let seed = hasher.finish() as usize;
        // Simple Fisher-Yates shuffle
        for i in (1..models.len()).rev() {
            let j = seed % (i + 1);
            models.swap(i, j);
        }

        let timeout = std::env::var("OPENROUTER_TIMEOUT")
            .unwrap_or_else(|_| "30".to_string())
            .parse()
            .unwrap_or(30);

        Some(RouterConfig {
            api_key,
            models,
            timeout,
            blacklist: Arc::new(Mutex::new(HashSet::new())),
        })
    }

    pub fn select_model(&self, request_type: RequestType) -> Option<String> {
        let blacklist = self.blacklist.lock().unwrap();
        let available: Vec<&String> = self
            .models
            .iter()
            .filter(|m| !blacklist.contains(*m))
            .collect();
        drop(blacklist);

        if available.is_empty() {
            return None;
        }

        match request_type {
            RequestType::Complex => Some(available[0].clone()),
            RequestType::Simple => Some(available[available.len() - 1].clone()),
        }
    }

    pub fn blacklist_model(&self, model: &str) {
        let mut blacklist = self.blacklist.lock().unwrap();
        blacklist.insert(model.to_string());
        let blacklist_clone = self.blacklist.clone();
        let model_owned = model.to_string();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(15 * 60));
            let mut bl = blacklist_clone.lock().unwrap();
            bl.remove(&model_owned);
        });
    }
}

pub fn analyze_request(prompt_len: usize, tool_count: usize, turn: usize) -> RequestType {
    if turn == 0 {
        return RequestType::Complex;
    }
    if prompt_len > 500 || tool_count > 0 {
        return RequestType::Complex;
    }
    RequestType::Simple
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_model_complex() {
        let config = RouterConfig {
            api_key: "test".to_string(),
            models: vec!["model-a".to_string(), "model-b".to_string()],
            timeout: 60,
            blacklist: Arc::new(Mutex::new(HashSet::new())),
        };
        assert_eq!(
            config.select_model(RequestType::Complex),
            Some("model-a".to_string())
        );
    }

    #[test]
    fn test_select_model_simple() {
        let config = RouterConfig {
            api_key: "test".to_string(),
            models: vec!["model-a".to_string(), "model-b".to_string()],
            timeout: 60,
            blacklist: Arc::new(Mutex::new(HashSet::new())),
        };
        assert_eq!(
            config.select_model(RequestType::Simple),
            Some("model-b".to_string())
        );
    }

    #[test]
    fn test_analyze_first_turn() {
        assert_eq!(analyze_request(100, 0, 0), RequestType::Complex);
    }

    #[test]
    fn test_analyze_long_prompt() {
        assert_eq!(analyze_request(600, 0, 1), RequestType::Complex);
    }

    #[test]
    fn test_analyze_with_tools() {
        assert_eq!(analyze_request(50, 1, 1), RequestType::Complex);
    }

    #[test]
    fn test_analyze_simple() {
        assert_eq!(analyze_request(50, 0, 1), RequestType::Simple);
    }
}

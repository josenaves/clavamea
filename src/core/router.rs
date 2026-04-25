use serde::{Deserialize, Serialize};

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
}

impl RouterConfig {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("OPENROUTER_API_KEY").ok()?;
        let models_str = std::env::var("OPENROUTER_MODELS")
            .unwrap_or_else(|_| "google/gemini-2.0-flash,openai/gpt-4o-mini".to_string());
        let models: Vec<String> = models_str.split(',').map(|s| s.trim().to_string()).collect();
        let timeout = std::env::var("OPENROUTER_TIMEOUT")
            .unwrap_or_else(|_| "60".to_string())
            .parse()
            .unwrap_or(60);
        
        Some(RouterConfig { api_key, models, timeout })
    }

    pub fn select_model(&self, request_type: RequestType) -> &str {
        match request_type {
            RequestType::Complex => self.models.first().map(|s| s.as_str()).unwrap_or("google/gemini-2.0-flash"),
            RequestType::Simple => self.models.last().map(|s| s.as_str()).unwrap_or("openai/gpt-4o-mini"),
        }
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
        };
        assert_eq!(config.select_model(RequestType::Complex), "model-a");
    }

    #[test]
    fn test_select_model_simple() {
        let config = RouterConfig {
            api_key: "test".to_string(),
            models: vec!["model-a".to_string(), "model-b".to_string()],
            timeout: 60,
        };
        assert_eq!(config.select_model(RequestType::Simple), "model-b");
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
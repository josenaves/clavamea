# OpenRouter Multi-Model Router Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implementar roteamento inteligente entre modelos gratuitos do OpenRouter com fallback automático.

**Architecture:** Novo módulo `router.rs` com lista de modelos + fallback. Engine integra automaticamente se `OPENROUTER_API_KEY` presente. Request classification: prompt > 500 chars OU tools > 0 = Complex → primeiro modelo; caso contrário = Simple → último modelo.

**Tech Stack:** Rust, reqwest, environment variables

---

## Task 1: Create router.rs

**Files:**
- Create: `src/core/router.rs`

### Step 1: Create router.rs with RequestType enum and RouterConfig

- [ ] **Step 1: Create router.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestType {
    Complex,  // prompt > 500 chars OU tools > 0
    Simple,  // prompt <= 500 chars E tools == 0
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
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test router -- --test-threads=1`
Expected: PASS (2 tests)

- [ ] **Step 3: Commit**

```bash
git add src/core/router.rs
git commit -m "feat: add router module with RequestType and RouterConfig"
```

---

## Task 2: Add analyze_request function

**Files:**
- Modify: `src/core/router.rs` (add analyze_request)

### Step 1: Add analyze_request helper

- [ ] **Step 1: Add analyze_request function**

Adicionar em `router.rs` após `select_model`:

```rust
pub fn analyze_request(prompt_len: usize, tool_count: usize, turn: usize) -> RequestType {
    // First turn always uses complex model
    if turn == 0 {
        return RequestType::Complex;
    }
    // Complex if prompt > 500 chars OR has tools
    if prompt_len > 500 || tool_count > 0 {
        return RequestType::Complex;
    }
    RequestType::Simple
}

#[cfg(test)]
mod tests {
    use super::*;

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
```

- [ ] **Step 2: Run tests**

Run: `cargo test router -- --test-threads=1`
Expected: PASS (6 tests)

- [ ] **Step 3: Commit**

```bash
git add src/core/router.rs
git commit -m "feat: add analyze_request function"
```

---

## Task 3: Integrate router in engine.rs

**Files:**
- Modify: `src/core/engine.rs`
- Test: `src/core/engine.rs` (add test)

### Step 1: Add router config to EngineConfig

- [ ] **Step 1: Modify EngineConfig to include router**

Modify: `src/core/engine.rs:17-27`

```rust
pub struct EngineConfig {
    pub api_url: String,
    pub api_key: String,
    pub model: String,
    pub model_pro: Option<String>,
    pub model_flash: Option<String>,
    pub max_tokens: u32,
    pub temperature: f32,
    pub storage: Arc<MemoryStorage>,
    pub allowed_paths: Arc<tokio::sync::RwLock<Vec<String>>,
    pub router: Option<RouterConfig>,  // <- Add this
}
```

- [ ] **Step 2: Add analyze_request call in generate method**

Modify lines 115-116 in `engine.rs` to add router logic:

```rust
// New Logic: Check if we should use router
let model = if let Some(router_config) = &self.config.router {
    let prompt_len = msgs.iter()
        .filter_map(|m| m.get("content").and_then(|c| c.as_str()))
        .map(|s| s.len())
        .sum::<usize>();
    let tool_count = tools.len();
    let request_type = crate::core::router::analyze_request(prompt_len, tool_count, 0);
    router_config.select_model(request_type).to_string()
} else {
    model_override.unwrap_or(&self.config.model).to_string()
};
```

- [ ] **Step 3: Register router module**

Adicionar em `src/core/mod.rs`:

```rust
pub mod router;
```

- [ ] **Step 4: Run cargo check**

Run: `cargo check`
Expected: Compiles without errors

- [ ] **Step 5: Commit**

```bash
git add src/core/engine.rs src/core/mod.rs
git commit -m "feat: integrate router in engine"
```

---

## Task 4: Add fallback logic in engine.rs

**Files:**
- Modify: `src/core/engine.rs`

### Step 1: Implement execute_with_fallback

- [ ] **Step 1: Add execute_with_fallback to engine.rs**

Modify the `generate` method to try multiple models on 429:

```rust
// After the API call (around line 148-154), replace error handling:

let mut last_error = None;
let tried_models: Vec<String> = vec![];

// If router fallback is enabled
if let Some(router_config) = &self.config.router {
    let models = &router_config.models;
    for model_attempt in models.iter() {
        payload["model"] = serde_json::json!(model_attempt);
        
        let res = self.client.post(&endpoint).bearer_auth(&router_config.api_key)...;
        
        if res.status() == 429 {
            tracing::warn!("Model {} rate limited, trying next", model_attempt);
            continue;
        }
        
        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            tracing::error!("LLM API error {}: {}", status, text);
            last_error = Some(anyhow::anyhow!("LLM API error {}: {}", status, text));
            continue;
        }
        
        // Success!
        break;
    }
}
```

**Nota:** Esta implementação requer refatorar a lógica atual de API call para um loop. O código exato depende da estrutura atual. Vamos expandir isso na Task.

- [ ] **Step 2: Run tests**

Run: `cargo test -- --test-threads=1`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git commit -m "feat: add fallback logic for router"
```

---

## Task 5: Update .env.example

**Files:**
- Modify: `.env.example`

### Step 1: Add new env vars

- [ ] **Step 1: Add OpenRouter env vars**

```env
# OpenRouter (alternative to DeepSeek)
OPENROUTER_API_KEY=sk-or-v1-xxx
OPENROUTER_MODELS=google/gemini-2.0-flash,openai/gpt-4o-mini,anthropic/claude-3-haiku-20240307
OPENROUTER_TIMEOUT=60
```

- [ ] **Step 2: Commit**

```bash
git add .env.example
git commit -m "docs: add OpenRouter env vars to .env.example"
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | router.rs com RouterConfig | Created |
| 2 | analyze_request function | Modified |
| 3 | Integrate router in engine | Modified |
| 4 | Fallback logic | Modified |
| 5 | .env.example | Modified |

Total esperado: ~5 commits
//! ClavaMea - Sovereign, private AI assistant.
//!
//! Entry point for the Telegram + WhatsApp bot application.

#![allow(dead_code)]
#![allow(clippy::collapsible_if)]

mod bot;
mod core;
mod db;
mod i18n;
mod whatsapp;

use std::env;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use dotenv::dotenv;
use teloxide::dispatching::Dispatcher;
use teloxide::dptree;
use tracing::{error, info, warn};

use crate::bot::{router, state::AppState};
use crate::core::engine::{Engine, EngineConfig};
use crate::db::connection;
use crate::i18n::bundle::BundleManager;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv().ok();

    // Initialize structured logging
    tracing_subscriber::fmt::init();

    info!("Starting ClavaMea v{}...", env!("CARGO_PKG_VERSION"));

    // Load environment variables
    let telegram_token =
        env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN must be set in .env file");
    let owner_id = env::var("OWNER_ID")
        .expect("OWNER_ID must be set in .env file")
        .parse::<i64>()
        .expect("OWNER_ID must be a valid integer");
    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data/clavamea.db".to_string());
    let locales_dir = env::var("LOCALES_DIR").unwrap_or_else(|_| "./locales".to_string());
    let max_conversation_length = env::var("MAX_CONVERSATION_LENGTH")
        .unwrap_or_else(|_| "20".to_string())
        .parse::<usize>()
        .expect("MAX_CONVERSATION_LENGTH must be a valid integer");

    // LLM configuration (optional for MVP)
    let llm_api_url = env::var("LLM_API_URL").ok();
    let llm_api_key = env::var("LLM_API_KEY").ok();
    let llm_model = env::var("LLM_MODEL").unwrap_or_else(|_| "deepseek-chat".to_string());
    let llm_model_pro = env::var("LLM_MODEL_PRO").ok();
    let llm_model_flash = env::var("LLM_MODEL_FLASH").ok();
    let llm_max_tokens = env::var("LLM_MAX_TOKENS")
        .unwrap_or_else(|_| "4096".to_string())
        .parse::<u32>()
        .expect("LLM_MAX_TOKENS must be a valid integer");
    let llm_temperature = env::var("LLM_TEMPERATURE")
        .unwrap_or_else(|_| "0.7".to_string())
        .parse::<f32>()
        .expect("LLM_TEMPERATURE must be a valid float");

    let allowed_paths_raw = env::var("ALLOWED_ORGANIZE_PATHS").unwrap_or_default();
    let allowed_paths: Vec<String> = allowed_paths_raw
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let dynamic_allowed_paths = Arc::new(tokio::sync::RwLock::new(allowed_paths));

    info!("Environment loaded. Owner ID: {}", owner_id);

    // Initialize database connection pool
    info!("Connecting to database...");
    let db_pool = connection::create_pool(&database_url).await?;
    info!("Database connection pool created.");

    // Run migrations
    info!("Running database migrations...");
    if let Err(e) = connection::run_migrations(&db_pool).await {
        error!("Failed to run migrations: {}", e);
        return Err(e);
    }
    info!("Database migrations completed.");

    // Seed owner
    if let Err(e) = crate::db::queries::ensure_owner_exists(&db_pool, owner_id).await {
        error!("Failed to seed owner in database: {}", e);
    }

    // Load i18n bundles
    info!("Loading i18n bundles from {}...", locales_dir);
    let locales_path = Path::new(&locales_dir);
    let i18n_manager = BundleManager::new(locales_path, "en")?;
    let i18n = Arc::new(i18n_manager);
    info!("i18n bundles loaded.");

    // Initialize LLM engine
    info!("Initializing LLM engine...");
    let memory_dir = env::var("MEMORY_DIR").unwrap_or_else(|_| "./memory".to_string());
    let storage = Arc::new(
        crate::core::storage::MemoryStorage::new(&memory_dir)
            .expect("Failed to initialize memory storage"),
    );

    let engine = if let (Some(api_url), Some(api_key)) = (llm_api_url, llm_api_key) {
        let config = EngineConfig {
            api_url,
            api_key,
            model: llm_model,
            model_pro: llm_model_pro.clone(),
            model_flash: llm_model_flash.clone(),
            max_tokens: llm_max_tokens,
            temperature: llm_temperature,
            storage: storage.clone(),
            allowed_paths: dynamic_allowed_paths.clone(),
            router: crate::core::router::RouterConfig::from_env(),
        };
        match Engine::new(config) {
            Ok(engine) => {
                info!("LLM engine initialized with API.");
                Arc::new(engine)
            }
            Err(e) => {
                warn!("Failed to initialize LLM engine: {}. Using placeholder.", e);
                Arc::new(
                    Engine::new(EngineConfig {
                        api_url: "placeholder".to_string(),
                        api_key: "placeholder".to_string(),
                        model: "placeholder".to_string(),
                        model_pro: None,
                        model_flash: None,
                        max_tokens: 4096,
                        temperature: 0.7,
                        storage: storage.clone(),
                        allowed_paths: dynamic_allowed_paths.clone(),
                        router: None,
                    })
                    .expect("Failed to init placeholder engine"),
                )
            }
        }
    } else {
        warn!("LLM API configuration not found. Using placeholder engine.");
        Arc::new(
            Engine::new(EngineConfig {
                api_url: "placeholder".to_string(),
                api_key: "placeholder".to_string(),
                model: "placeholder".to_string(),
                model_pro: None,
                model_flash: None,
                max_tokens: 4096,
                temperature: 0.7,
                storage: storage.clone(),
                allowed_paths: dynamic_allowed_paths.clone(),
                router: None,
            })
            .expect("Failed to init placeholder engine"),
        )
    };

    // Initialize RAG manager
    info!("Initializing RAG manager...");
    let rag_manager = crate::core::RagManager::new(db_pool.clone())?;
    let rag = Arc::new(rag_manager);
    info!("RAG manager initialized.");

    // Initialize Wasm runtime
    info!("Initializing Wasm runtime...");
    let wasm_runtime = crate::core::wasm::WasmRuntime::new()?;
    let wasm = Arc::new(wasm_runtime);
    info!("Wasm runtime initialized.");

    // Create Telegram bot
    info!("Creating Telegram bot...");
    let bot = teloxide::Bot::new(telegram_token);

    // Create application state
    let state = AppState::new(
        db_pool,
        engine.clone(),
        i18n.clone(),
        rag,
        wasm,
        owner_id,
        max_conversation_length,
        bot.clone(),
    );

    // Initialize and spawn Task Scheduler
    info!("Starting Task Scheduler...");
    let scheduler_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::bot::scheduler::run_scheduler(scheduler_state).await {
            error!("Task Scheduler error: {}", e);
        }
    });

    // Initialize and spawn WhatsApp webhook server (if configured)
    let whatsapp_bridge_url = env::var("WHATSAPP_BRIDGE_URL").ok();
    if let Some(bridge_url) = whatsapp_bridge_url {
        let webhook_port: u16 = env::var("WHATSAPP_WEBHOOK_PORT")
            .unwrap_or_else(|_| "8081".to_string())
            .parse()
            .expect("WHATSAPP_WEBHOOK_PORT must be a valid port number");

        info!(
            "Starting WhatsApp webhook server on port {}...",
            webhook_port
        );
        info!("WhatsApp bridge URL: {}", bridge_url);

        let wa_sender = crate::whatsapp::sender::WhatsAppSender::new(&bridge_url);
        let wa_state = crate::whatsapp::webhook::WhatsAppWebhookState {
            app_state: state.clone(),
            sender: wa_sender,
        };
        let wa_router = crate::whatsapp::webhook::create_router(wa_state);

        tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", webhook_port))
                .await
                .expect("Failed to bind WhatsApp webhook port");
            info!("WhatsApp webhook server listening on port {}", webhook_port);
            if let Err(e) = axum::serve(listener, wa_router).await {
                error!("WhatsApp webhook server error: {}", e);
            }
        });
    } else {
        info!("WhatsApp integration disabled (WHATSAPP_BRIDGE_URL not set).");
    }

    // Set up the Telegram dispatcher with our schema
    info!("Setting up Telegram dispatcher...");
    Dispatcher::builder(bot, router::schema())
        .dependencies(dptree::deps![state])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    info!("Shutting down...");
    Ok(())
}

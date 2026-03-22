//! Background task scheduler for recurring jobs.

use crate::bot::AppState;
use crate::core::memory::Role;
use crate::core::renderer::Renderer;
use chrono::{Datelike, Local, Timelike};
use std::time::Duration;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::Requester;
use tracing::{debug, error, info};

/// Run the background scheduler loop.
pub async fn run_scheduler(state: AppState) -> anyhow::Result<()> {
    info!("Scheduler background loop started.");

    let mut interval = tokio::time::interval(Duration::from_secs(60));

    loop {
        interval.tick().await;

        let now = Local::now();
        let time_str = now.format("%H:%M").to_string();
        let weekday = now.weekday().to_string().to_uppercase(); // e.g. "MON"

        debug!("Checking scheduled tasks for {} ({})", time_str, weekday);

        // Query database for tasks due at this time
        if let Err(e) = process_due_tasks(&state, &time_str, &weekday).await {
            error!("Error processing scheduled tasks: {}", e);
        }
    }
}

async fn process_due_tasks(state: &AppState, time_str: &str, weekday: &str) -> anyhow::Result<()> {
    // 1. Fetch due tasks from DB
    let tasks = crate::db::queries::get_due_schedules(&state.db_pool, time_str, weekday).await?;

    for task in tasks {
        info!(
            "Executing scheduled task: {} (Type: {})",
            task.id, task.task_type
        );

        match task.task_type.as_str() {
            "bovespa_clipping" => {
                let state_clone = state.clone();
                let user_id = task.user_id;
                tokio::spawn(async move {
                    if let Err(e) = execute_bovespa_clipping(state_clone, user_id).await {
                        error!("Bovespa clipping failed for user {}: {}", user_id, e);
                    }
                });
            }
            _ => {
                error!("Unknown task type: {}", task.task_type);
            }
        }

        // Update last run
        let _ = crate::db::queries::update_schedule_last_run(&state.db_pool, task.id).await;
    }

    Ok(())
}

async fn execute_bovespa_clipping(state: AppState, user_id: i64) -> anyhow::Result<()> {
    info!("Running Bovespa clipping for user {}", user_id);

    // Use the engine to generate the clipping with web search capabilities
    let tools = vec![crate::core::tools::Tool::WebSearch];

    // Create a memory object with a high-level system instruction for the clipping
    let mut memory = crate::core::memory::ConversationMemory::new(user_id, 5);
    memory.add_message(crate::core::memory::Message {
        role: Role::System,
        content: Some("Você é um analista financeiro. Gere um clipping diário do fechamento da Bovespa (IBovespa) focado em: \
                  índice atual, maiores altas, maiores baixas e cotação do dólar. Use suas ferramentas de busca para obter dados de hoje.".to_string()),
        tool_calls: None,
        tool_call_id: None,
    });

    let response = state
        .engine
        .generate(user_id, &memory, &tools, "pt")
        .await?;

    match response {
        crate::core::LLMResponse::Text(text) => {
            let renderer = crate::core::renderer::TelegramRenderer::new();
            let rendered = renderer.render(&text);

            crate::bot::utils::send_chunked_message(&state.bot, teloxide::types::ChatId(user_id), &rendered).await?;
        }
        _ => {
            error!("LLM returned tool calls for a scheduled background task. Skipping.");
        }
    }

    Ok(())
}

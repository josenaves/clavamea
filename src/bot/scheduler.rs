//! Background task scheduler for recurring jobs.

use crate::bot::AppState;
use crate::core::memory::Role;
use crate::core::renderer::Renderer;
use chrono::{Datelike, Local, Timelike};
use std::future::Future;
use std::time::Duration;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::Requester;
use tracing::{error, info};

/// Run the background scheduler loop.
pub async fn run_scheduler(state: AppState) -> anyhow::Result<()> {
    info!("Scheduler background loop started.");

    let mut interval = tokio::time::interval(Duration::from_secs(60));

    loop {
        interval.tick().await;

        let now = Local::now();
        let time_str = now.format("%H:%M").to_string();
        let weekday = now.weekday().to_string().to_uppercase(); // e.g. "MON"

        tracing::info!("Scheduler tick: {} ({})", time_str, weekday);

        // Query database for tasks due at this time
        if let Err(e) = process_due_tasks(&state, &time_str, &weekday).await {
            error!("Error processing scheduled tasks: {}", e);
        }
    }
}

async fn process_due_tasks(state: &AppState, time_str: &str, weekday: &str) -> anyhow::Result<()> {
    // Fetch users so we know their timezones
    let users = crate::db::queries::list_users(&state.db_pool).await?;

    for user in &users {
        let tz = user.timezone.as_deref().unwrap_or("UTC");
        let tasks =
            crate::db::queries::get_due_schedules(&state.db_pool, time_str, weekday, tz).await?;

        for task in tasks {
            info!(
                "Executing scheduled task: {} (Type: {})",
                task.id, task.task_type
            );

            match task.task_type.as_str() {
                "bovespa_clipping" => {
                    let state_clone = state.clone();
                    let user_id = task.user_id;
                    let schedule_id = task.id;
                    tokio::spawn(async move {
                        if let Err(e) =
                            execute_bovespa_clipping(state_clone, user_id, schedule_id).await
                        {
                            error!("Bovespa clipping failed for user {}: {}", user_id, e);
                        }
                    });
                }
                "reminder" => {
                    let state_clone = state.clone();
                    let user_id = task.user_id;
                    let payload = task.payload.clone();
                    let schedule_id = task.id;
                    let is_one_time = is_one_time_expr(&task.cron_expr);

                    tokio::spawn(async move {
                        if let Err(e) = execute_reminder(
                            state_clone,
                            user_id,
                            payload,
                            schedule_id,
                            is_one_time,
                        )
                        .await
                        {
                            error!("Reminder failed for user {}: {}", user_id, e);
                        }
                    });
                }
                _ => {
                    error!("Unknown task type: {}", task.task_type);
                }
            }
        }
    }

    Ok(())
}

async fn execute_bovespa_clipping(
    state: AppState,
    user_id: i64,
    schedule_id: i64,
) -> anyhow::Result<()> {
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
        .generate(user_id, &memory, &tools, "pt", None, None)
        .await?;

    match response {
        crate::core::LLMResponse::Text(text) => {
            let renderer = crate::core::renderer::TelegramMarkdownV2Renderer::new();
            let rendered = renderer.render(&text);

            crate::bot::utils::send_chunked_message(
                &state.bot,
                teloxide::types::ChatId(user_id),
                &rendered,
            )
            .await?;

            // Mark as run only after successful delivery
            crate::db::queries::update_schedule_last_run(&state.db_pool, schedule_id).await?;
        }
        _ => {
            error!("LLM returned tool calls for a scheduled background task. Skipping.");
        }
    }

    Ok(())
}
/// Returns `true` if the cron expression represents a one-time event
/// (starts with a date like "YYYY-MM-DD") rather than a recurring schedule.
pub fn is_one_time_expr(cron_expr: &str) -> bool {
    cron_expr
        .split_whitespace()
        .next()
        .map(|p| {
            p.len() == 10
                && p.chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
        })
        .unwrap_or(false)
}

/// Retries an async operation up to `max_attempts` times with exponential backoff.
/// Delay between attempts doubles each time: 1s, 2s, 4s, ...
/// Logs retry attempts via `info!`.
async fn send_with_retry<F, Fut>(
    schedule_id: i64,
    max_attempts: u32,
    mut f: F,
) -> anyhow::Result<()>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    let mut last_error = None;
    for attempt in 0..max_attempts {
        if attempt > 0 {
            let delay = Duration::from_secs(1u64 << attempt);
            info!(
                "Retry attempt {}/{} for schedule {} (waiting {}s)...",
                attempt + 1,
                max_attempts,
                schedule_id,
                delay.as_secs()
            );
            tokio::time::sleep(delay).await;
        }

        match f().await {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_error = Some(e);
            }
        }
    }

    Err(anyhow::anyhow!(
        "Operation failed after {} attempts: {:?}",
        max_attempts,
        last_error
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn test_is_one_time_expr_valid_date() {
        assert!(is_one_time_expr("2026-04-25 08:00"));
        assert!(is_one_time_expr("2026-12-01 14:30"));
        assert!(is_one_time_expr("1999-01-01 00:00"));
    }

    #[test]
    fn test_is_one_time_expr_recurring_hhmm_only() {
        assert!(!is_one_time_expr("08:00"));
        assert!(!is_one_time_expr("22:30"));
    }

    #[test]
    fn test_is_one_time_expr_recurring_with_days() {
        assert!(!is_one_time_expr("08:00 MON-FRI"));
        assert!(!is_one_time_expr("17:10 MON,WED,FRI"));
        assert!(!is_one_time_expr("09:00 MON"));
    }

    #[test]
    fn test_is_one_time_expr_edge_cases() {
        // Empty string
        assert!(!is_one_time_expr(""));
        // Only spaces
        assert!(!is_one_time_expr("   "));
        // Date-like but only 9 chars (e.g. "2026-04-1" — should reject)
        assert!(!is_one_time_expr("2026-04-1 08:00"));
        // 10 chars starting with letter (e.g. "ABCD-01-01")
        assert!(!is_one_time_expr("ABCD-01-01 08:00"));
        // MON-FRI contains '-' but starts with "MON" (3 chars), not a date
        assert!(!is_one_time_expr("08:00 MON-FRI"));
    }

    #[tokio::test]
    async fn test_send_with_retry_success_first_attempt() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = calls.clone();

        let result = send_with_retry(1, 3, || {
            let c = calls_clone.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "should succeed on first attempt"
        );
    }

    #[tokio::test]
    async fn test_send_with_retry_success_after_failures() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = calls.clone();

        let result = send_with_retry(2, 3, || {
            let c = calls_clone.clone();
            async move {
                let attempt = c.fetch_add(1, Ordering::SeqCst) + 1;
                if attempt < 3 {
                    Err(anyhow::anyhow!("simulated failure"))
                } else {
                    Ok(())
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(
            calls.load(Ordering::SeqCst),
            3,
            "should succeed on 3rd attempt"
        );
    }

    #[tokio::test]
    async fn test_send_with_retry_all_attempts_fail() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = calls.clone();

        let result = send_with_retry(3, 3, || {
            let c = calls_clone.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Err(anyhow::anyhow!("simulated failure"))
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(
            calls.load(Ordering::SeqCst),
            3,
            "should exhaust all 3 attempts"
        );
    }

    #[tokio::test]
    async fn test_send_with_retry_single_attempt() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = calls.clone();

        let result = send_with_retry(4, 1, || {
            let c = calls_clone.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Err(anyhow::anyhow!("simulated failure"))
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "max_attempts=1 should try only once"
        );
    }
}

async fn execute_reminder(
    state: AppState,
    user_id: i64,
    payload: Option<String>,
    schedule_id: i64,
    is_one_time: bool,
) -> anyhow::Result<()> {
    info!("Sending reminder to user {}", user_id);

    let message =
        payload.unwrap_or_else(|| "Você tem um lembrete agendado para agora.".to_string());

    let renderer = crate::core::renderer::TelegramMarkdownV2Renderer::new();
    let rendered = renderer.render(&message);

    let bot = state.bot.clone();
    let pool = state.db_pool.clone();

    send_with_retry(schedule_id, 3, || {
        let rendered = rendered.clone();
        let bot = bot.clone();
        async move {
            crate::bot::utils::send_chunked_message(
                &bot,
                teloxide::types::ChatId(user_id),
                &rendered,
            )
            .await
            .map_err(anyhow::Error::from)
        }
    })
    .await?;

    // Mark as run only after successful delivery
    crate::db::queries::update_schedule_last_run(&pool, schedule_id).await?;

    if is_one_time {
        info!("Deleting one-time reminder task: {}", schedule_id);
        if let Err(e) = crate::db::queries::delete_schedule(&pool, schedule_id).await {
            error!("Failed to delete one-time reminder {}: {}", schedule_id, e);
        }
    }

    Ok(())
}

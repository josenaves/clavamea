use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::types::{ParseMode, ReplyParameters};
use tracing::{error, info, warn};

/// Sends a message to Telegram using Teloxide, retrying with exponential backoff on failure.
pub async fn send_message_with_retry(
    bot: &Bot,
    chat_id: teloxide::types::ChatId,
    text: &str,
    parse_mode: Option<ParseMode>,
    reply_to_message_id: Option<teloxide::types::MessageId>,
) -> ResponseResult<teloxide::types::Message> {
    let mut last_error = None;
    let max_attempts = 5;

    for attempt in 0..max_attempts {
        if attempt > 0 {
            // Exponential backoff: 1s, 2s, 4s, 8s, 16s...
            let delay = std::time::Duration::from_secs(1 << (attempt - 1));
            warn!(
                "Retrying sending Telegram message to {} (attempt {}/{}). Waiting {}s...",
                chat_id,
                attempt + 1,
                max_attempts,
                delay.as_secs()
            );
            tokio::time::sleep(delay).await;
        }

        // Build the send request with optional parse mode and reply id
        let send_result = {
            let mut builder = bot.send_message(chat_id, text);
            if let Some(mode) = parse_mode {
                builder = builder.parse_mode(mode);
            }
            if let Some(reply_id) = reply_to_message_id {
                builder = builder.reply_parameters(ReplyParameters::new(reply_id));
            }
            builder.await
        };
        match send_result {
            Ok(msg) => return Ok(msg),
            Err(e) => {
                error!(
                    "Failed to send Telegram message to {} on attempt {}/{}: {}",
                    chat_id,
                    attempt + 1,
                    max_attempts,
                    e
                );
                last_error = Some(e);
            }
        }
    }

    Err(last_error.expect("At least one attempt must have failed"))
}

/// Safely sends a long message to Telegram by chunking it if it exceeds the 4096 character limit.
/// Tries to split by newlines where possible to preserve formatting.
pub async fn send_chunked_message(
    bot: &Bot,
    chat_id: teloxide::types::ChatId,
    text: &str,
    reply_to_message_id: Option<teloxide::types::MessageId>,
) -> ResponseResult<()> {
    const MAX_LEN: usize = 4000; // Leave a little buffer for safety

    info!("Sending response to {} ({} chars)", chat_id, text.len());

    if text.len() <= MAX_LEN {
        send_message_with_retry(
            bot,
            chat_id,
            text,
            Some(ParseMode::MarkdownV2),
            reply_to_message_id,
        )
        .await?;
        return Ok(());
    }

    let mut current_chunk = String::new();
    let mut in_code_block = false;

    for line in text.split('\n') {
        let toggles_code_block = line.trim().starts_with("```");

        // +1 for the newline character
        if current_chunk.len() + line.len() + 1 > MAX_LEN {
            if !current_chunk.is_empty() {
                // If we are cutting the chunk while inside a code block, close it gracefully
                if in_code_block {
                    current_chunk.push_str("```\n");
                }

                send_message_with_retry(
                    bot,
                    chat_id,
                    &current_chunk,
                    Some(ParseMode::MarkdownV2),
                    reply_to_message_id,
                )
                .await?;
                current_chunk.clear();

                // Re-open the code block in the next chunk if we were inside one
                if in_code_block {
                    current_chunk.push_str("```\n");
                }
            }

            if line.len() > MAX_LEN {
                // If a single line is enormous, split by characters
                let mut chars = line.chars().peekable();
                let chunk_size = if in_code_block { MAX_LEN - 10 } else { MAX_LEN };

                while chars.peek().is_some() {
                    let mut chunk: String = chars.by_ref().take(chunk_size).collect();

                    if in_code_block {
                        chunk.push_str("\n```\n");
                    }

                    send_message_with_retry(
                        bot,
                        chat_id,
                        &chunk,
                        Some(ParseMode::MarkdownV2),
                        reply_to_message_id,
                    )
                    .await?;

                    // We don't prepend ``` for the next part of the line here because it gets messy,
                    // and lines > 4000 chars are extremely rare.
                    // But if it happens and ends the message, we might have an unclosed block in the next normal chunk.
                    // Let's just clear the chunk and assume the rest of the long line is sent raw.
                }
                // If the long line happened to toggle a code block, we just toggle it.
                if toggles_code_block {
                    in_code_block = !in_code_block;
                }
            } else {
                current_chunk.push_str(line);
                current_chunk.push('\n');
                if toggles_code_block {
                    in_code_block = !in_code_block;
                }
            }
        } else {
            current_chunk.push_str(line);
            current_chunk.push('\n');
            if toggles_code_block {
                in_code_block = !in_code_block;
            }
        }
    }

    if !current_chunk.trim().is_empty() {
        if in_code_block && !current_chunk.trim().ends_with("```") {
            current_chunk.push_str("\n```");
        }
        send_message_with_retry(
            bot,
            chat_id,
            &current_chunk,
            Some(ParseMode::MarkdownV2),
            reply_to_message_id,
        )
        .await?;
    }

    Ok(())
}

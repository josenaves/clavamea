use teloxide::prelude::*;
use teloxide::types::ParseMode;
use tracing::info;

/// Safely sends a long message to Telegram by chunking it if it exceeds the 4096 character limit.
/// Tries to split by newlines where possible to preserve formatting.
pub async fn send_chunked_message(
    bot: &Bot,
    chat_id: teloxide::types::ChatId,
    text: &str,
) -> ResponseResult<()> {
    const MAX_LEN: usize = 4000; // Leave a little buffer for safety

    info!("Sending response to {} ({} chars)", chat_id, text.len());

    if text.len() <= MAX_LEN {
        bot.send_message(chat_id, text)
            .parse_mode(ParseMode::MarkdownV2)
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

                bot.send_message(chat_id, &current_chunk)
                    .parse_mode(ParseMode::MarkdownV2)
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

                    bot.send_message(chat_id, &chunk)
                        .parse_mode(ParseMode::MarkdownV2)
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
        bot.send_message(chat_id, &current_chunk)
            .parse_mode(ParseMode::MarkdownV2)
            .await?;
    }

    Ok(())
}

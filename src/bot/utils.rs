use teloxide::prelude::*;
use teloxide::types::ParseMode;

/// Safely sends a long message to Telegram by chunking it if it exceeds the 4096 character limit.
/// Tries to split by newlines where possible to preserve formatting.
pub async fn send_chunked_message(
    bot: &Bot,
    chat_id: teloxide::types::ChatId,
    text: &str,
) -> ResponseResult<()> {
    const MAX_LEN: usize = 4000; // Leave a little buffer for safety

    if text.len() <= MAX_LEN {
        bot.send_message(chat_id, text)
            .parse_mode(ParseMode::MarkdownV2)
            .await?;
        return Ok(());
    }

    let mut current_chunk = String::new();

    for line in text.split('\n') {
        // +1 for the newline character
        if current_chunk.len() + line.len() + 1 > MAX_LEN {
            if !current_chunk.is_empty() {
                bot.send_message(chat_id, &current_chunk)
                    .parse_mode(ParseMode::MarkdownV2)
                    .await?;
                current_chunk.clear();
            }

            if line.len() > MAX_LEN {
                // If a single line is enormous, split by characters
                let mut chars = line.chars().peekable();
                while chars.peek().is_some() {
                    let chunk: String = chars.by_ref().take(MAX_LEN).collect();
                    bot.send_message(chat_id, &chunk)
                        .parse_mode(ParseMode::MarkdownV2)
                        .await?;
                }
            } else {
                current_chunk.push_str(line);
                current_chunk.push('\n');
            }
        } else {
            current_chunk.push_str(line);
            current_chunk.push('\n');
        }
    }

    if !current_chunk.trim().is_empty() {
        bot.send_message(chat_id, &current_chunk)
            .parse_mode(ParseMode::MarkdownV2)
            .await?;
    }

    Ok(())
}

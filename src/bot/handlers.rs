//! Telegram message and command handlers.

use teloxide::prelude::*;
use teloxide::types::{Message as TgMessage, ParseMode};

use crate::bot::state::AppState;
use crate::core::{
    ConversationMemory, LLMResponse, Message as MemoryMessage, Renderer,
    TelegramMarkdownV2Renderer as TelegramRenderer, Tool, get_available_tools,
};
use crate::db::models::{Interaction, NewInteraction, User};

/// Current bot version. Bump this in Cargo.toml when deploying.
const BOT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Changelog shown when user requests via /changelog or /whatsnew.
const CHANGELOG: &str = r#"🆕 **O ClavaMea foi atualizado\!**

**v1\.11\.0 — Modelos NVIDIA Gratuitos**
• **NVIDIA NIM\!** Agora o ClavaMea suporta modelos gratuitos da NVIDIA \(como DeepSeek V3/R1\)\.
• **Seleção Inteligente\!** O bot decide automaticamente entre modelos PRO e FLASH baseado no turno e na complexidade da tarefa\.
• **Controle de Provedor\!** Escolha entre NVIDIA, OpenRouter ou DeepSeek via `LLM_PROVIDER`\.

**v1\.8\.0 — Integração WhatsApp**
• **WhatsApp\!** Agora o ClavaMea pode ser acessado pelo WhatsApp via ponte com o WhatsApp Web\.
• Mesmas ferramentas e inteligência do Telegram, agora no mensageiro mais popular do Brasil\.
• Controle de acesso unificado: aprovação de usuários funciona para ambos os canais\.

**v1\.7\.0 — YouTube Music Downloader**
• **Música no Telegram\!** Baixe qualquer música do YouTube diretamente para o seu chat\.
• O ClavaMea agora converte vídeos para MP3 de alta qualidade e envia o arquivo para você\.
• Limite: vídeos de até 10 minutos\.
• Ferramenta: `download_music`\.

**v1\.6\.0 — Agentic Self\-Evolving**
• **Modo Autônomo\!** O ClavaMea agora pode editar seu próprio código, realizar commits com o git e ler e responder a issues diretamente no Github\.
• Ferramentas: `edit_code`, `git_operate`, `github_read_issues`, e `github_update_issue`\.

**v1\.5\.1 — Gestão de Receitas**
• **Habilidade de Culinária\!** Importe receitas de links, arquivos ou texto\.
• O ClavaMea agora limpa o conteúdo de sites, removendo anúncios e distrações para focar no que importa\.
• Ferramentas: `fetch_url`, `save_recipe` e `list_recipes`\.

**v1\.5\.0 — Correção de Lembretes**
• Corrigido: lembretes agendados para datas específicas não eram disparados após as 21h \(bug de fuso horário UTC vs\. local\)\.
• Corrigido: lembretes recorrentes com padrão `MON\-FRI` eram deletados após a primeira execução\.
• Agora os agendamentos funcionam de forma confiável em qualquer horário do dia\!

**v1\.4\.0 — Gestão de Identidade**
• Agora você pode definir nomes para os usuários\!
• Comando /users agora mostra o nome real de cada pessoa\.
• Novo formato: `/approve \<id\> \<role\> \<nome\.\.\.\>`

**v1\.3\.0 — Agendamento de Tarefas \(Bovespa\)**
• Receba o fechamento da Bolsa às 17:10 \(SEG\-SEX\)\.
• Skill de Cálculo de Genética \(Hardy\-Weinberg\)\.

**v1\.2\.0 — Sistema Multi\-usuário**
• Controle de acesso por roles e dados isolados\."#;

/// Handle incoming text messages.
pub async fn handle_message(bot: Bot, msg: TgMessage, state: AppState) -> ResponseResult<()> {
    // Log the message
    tracing::info!(
        "Received message from {} ({}): {}",
        msg.chat.id,
        msg.from
            .as_ref()
            .map(|u| u.id.0.to_string())
            .unwrap_or_default(),
        msg.text().unwrap_or("[no text]")
    );

    // Extract user_id from the sender
    let user_id = match &msg.from {
        Some(u) => u.id.0 as i64,
        None => return Ok(()), // Ignore messages without a sender
    };

    let username = msg.from.as_ref().and_then(|u| u.username.as_deref());

    // Check if user is authorized in the database
    let user_record: Option<User> = crate::db::queries::get_user(&state.db_pool, user_id)
        .await
        .unwrap_or(None);

    let is_authorized = match user_record {
        Some(ref u) => u.authorized,
        None => {
            // Register new user as pending
            let _ =
                crate::db::queries::create_user_pending(&state.db_pool, user_id, username).await;

            // Notify the owner about the new pending user
            let owner_chat_id = teloxide::types::ChatId(state.owner_id);
            let admin_msg = format!(
                "🔔 **Novo usuário pendente!**\n\nID: `{}`\nNome: {}\n\nUse `/approve {} <papel> <nome>` para liberar.",
                user_id,
                username.unwrap_or("não informado"),
                user_id
            );
            let renderer = TelegramRenderer::new();
            let rendered_admin = renderer.render(&admin_msg);
            let _ = bot
                .send_message(owner_chat_id, rendered_admin)
                .parse_mode(ParseMode::MarkdownV2)
                .await;

            false
        }
    };

    let is_owner = user_id == state.owner_id;
    let is_admin = is_owner
        || user_record
            .as_ref()
            .map(|u: &User| u.is_admin())
            .unwrap_or(false);

    // Guard: Only authorized users can proceed, unless it's a special admin command from an admin
    if !is_authorized && !is_admin {
        bot.send_message(msg.chat.id, "🚫 **Acesso Restrito**\n\nDesculpe, você não tem permissão para usar este bot. Sua solicitação de acesso foi enviada para o administrador.").await?;
        return Ok(());
    }

    // Track user's version silently (no auto-changelog spam)
    if let Some(ref u) = user_record {
        if u.last_seen_version != BOT_VERSION {
            let _ =
                crate::db::queries::update_user_seen_version(&state.db_pool, user_id, BOT_VERSION)
                    .await;
        }
    }

    // Handle Admin Commands
    if is_admin {
        if let Some(text) = msg.text() {
            if text.starts_with("/changelog") || text.starts_with("/whatsnew") {
                let renderer = TelegramRenderer::new();
                let rendered_changelog = renderer.render(CHANGELOG);
                bot.send_message(msg.chat.id, rendered_changelog)
                    .parse_mode(ParseMode::MarkdownV2)
                    .await?;
                return Ok(());
            } else if text.starts_with("/users") {
                let users: Vec<User> = crate::db::queries::list_users(&state.db_pool)
                    .await
                    .unwrap_or_else(|_| Vec::new());
                let mut response = "👥 **Lista de Usuários:**\n\n".to_string();
                for u in users {
                    let status = if u.authorized { "✅" } else { "⏳" };
                    let display_name = u.full_name.clone().unwrap_or_else(|| {
                        u.username.clone().unwrap_or_else(|| "Novo".to_string())
                    });
                    response.push_str(&format!(
                        "{} ID: `{}` | {} | {}\n",
                        status, u.id, u.role, display_name
                    ));
                }

                let renderer = TelegramRenderer::new();
                let rendered = renderer.render(&response);
                bot.send_message(msg.chat.id, rendered)
                    .parse_mode(ParseMode::MarkdownV2)
                    .await?;
                return Ok(());
            } else if text.starts_with("/auth") || text.starts_with("/approve") {
                let parts: Vec<&str> = text.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(target_id) = parts[1].parse::<i64>() {
                        // Syntax: /approve <id> [role] [name...]
                        let mut role = "friend".to_string();
                        let mut name_start_idx = 2;

                        if parts.len() > 2 {
                            let possible_role = parts[2].to_lowercase();
                            if ["owner", "admin", "family", "friend", "subscriber"]
                                .contains(&possible_role.as_str())
                            {
                                role = possible_role;
                                name_start_idx = 3;
                            }
                        }

                        let full_name = if parts.len() > name_start_idx {
                            Some(parts[name_start_idx..].join(" "))
                        } else {
                            None
                        };

                        let _ = crate::db::queries::authorize_user_with_name(
                            &state.db_pool,
                            target_id,
                            &role,
                            full_name.as_deref(),
                        )
                        .await;

                        // Ensure user directory exists
                        let _ = state.engine.storage.ensure_user_files(target_id);

                        // Notify the admin
                        let name_display = full_name
                            .clone()
                            .unwrap_or_else(|| "ID ".to_string() + &target_id.to_string());
                        bot.send_message(
                            msg.chat.id,
                            format!("✅ Usuário `{}` autorizado como `{}`.", name_display, role),
                        )
                        .await?;

                        // Notify the target user
                        let welcome_msg = format!(
                            "🎉 **Acesso Liberado!**\n\nOlá! Sua solicitação de acesso ao ClavaMea foi aprovada como `{}`. Agora você pode interagir comigo e usar minhas ferramentas.",
                            role
                        );
                        let renderer = TelegramRenderer::new();
                        let rendered_welcome = renderer.render(&welcome_msg);
                        let _ = bot
                            .send_message(teloxide::types::ChatId(target_id), rendered_welcome)
                            .parse_mode(ParseMode::MarkdownV2)
                            .await;

                        return Ok(());
                    }
                }
            } else if text.starts_with("/deauth")
                || text.starts_with("/deny")
                || text.starts_with("/reject")
            {
                let parts: Vec<&str> = text.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(target_id) = parts[1].parse::<i64>() {
                        let _ =
                            crate::db::queries::deauthorize_user(&state.db_pool, target_id).await;
                        bot.send_message(
                            msg.chat.id,
                            format!("❌ Autorização removida para `{}`.", target_id),
                        )
                        .await?;
                        return Ok(());
                    }
                }
            }
        }
    }

    // Handle the actual text message
    if let Some(text) = msg.text() {
        let chat_id = msg.chat.id.0;
        let lang = "en"; // Default for MVP

        // Initialize renderer
        let renderer = TelegramRenderer::new();

        // Insert user interaction into DB
        let user_interaction = NewInteraction::user(chat_id, text.to_string(), lang);
        if let Err(e) =
            crate::db::queries::insert_interaction(&state.db_pool, &user_interaction).await
        {
            tracing::error!("Failed to save user interaction: {}", e);
        }

        // Send a "typing..." action so the user knows ClavaMea is thinking
        let _ = bot
            .send_chat_action(msg.chat.id, teloxide::types::ChatAction::Typing)
            .await;

        // Initialize memory for this conversation
        let history = crate::db::queries::get_recent_interactions(
            &state.db_pool,
            chat_id,
            state.max_conversation_length as u32,
        )
        .await
        .unwrap_or_else(|_| Vec::new());
        let mut memory =
            ConversationMemory::from_interactions(history, state.max_conversation_length);
        memory.add_message(MemoryMessage::user(text.to_string()));

        // Available tools for Phase 3 (WebSearch + FileReader + Memory + RAG)
        let tools = get_available_tools(3);
        let mut turn = 0;
        let max_turns = 20;

        loop {
            if turn >= max_turns {
                tracing::warn!("Max turns reached for user {}", user_id);
                bot.send_message(
                    msg.chat.id,
                    "I reached my maximum thinking limit for this turn.",
                )
                .await?;
                break;
            }

            tracing::info!("Calling LLM for user {} (turn {})", user_id, turn);
            let model_override = if turn == 0 && !tools.is_empty() {
                state.engine.config().model_pro.as_deref()
            } else if !tools.is_empty() {
                state.engine.config().model_flash.as_deref()
            } else {
                None
            };
            if let Some(m) = model_override {
                tracing::info!("  using model override: {}", m);
            }
            match state
                .engine
                .generate(
                    user_id,
                    &memory,
                    &tools,
                    lang,
                    user_record.as_ref().and_then(|u| u.timezone.as_deref()),
                    model_override,
                )
                .await
            {
                Ok(LLMResponse::Text(content)) => {
                    // Final text response
                    tracing::info!(
                        "LLM returned text for user {} ({} chars)",
                        user_id,
                        content.len()
                    );
                    let assistant_interaction =
                        NewInteraction::assistant(chat_id, content.clone(), lang);
                    if let Err(e) = crate::db::queries::insert_interaction(
                        &state.db_pool,
                        &assistant_interaction,
                    )
                    .await
                    {
                        tracing::error!("Failed to save assistant interaction: {}", e);
                    }

                    // Render Markdown to Telegram HTML
                    let rendered_content = renderer.render(&content);
                    crate::bot::utils::send_chunked_message(&bot, msg.chat.id, &rendered_content)
                        .await?;
                    break;
                }
                Ok(LLMResponse::ToolCalls(tool_calls)) => {
                    // LLM requested tool execution
                    tracing::info!(
                        "LLM requested {} tool(s) for user {}: {:?}",
                        tool_calls.len(),
                        user_id,
                        tool_calls
                            .iter()
                            .map(|t| t.function.name.as_str())
                            .collect::<Vec<_>>()
                    );
                    memory.add_message(MemoryMessage::tool_calls(tool_calls.clone()));

                    for tool_call in tool_calls {
                        let tool_name = tool_call.function.name.as_str();
                        let args: serde_json::Value =
                            match serde_json::from_str(&tool_call.function.arguments) {
                                Ok(v) => v,
                                Err(e) => {
                                    memory.add_message(MemoryMessage::tool_result(
                                        tool_call.id.clone(),
                                        format!("Invalid arguments: {}", e),
                                    ));
                                    continue;
                                }
                            };

                        tracing::info!("LLM requested tool: {}", tool_name);

                        let tool_option = Tool::from_name(tool_name);

                        if let Some(tool) = tool_option {
                            match tool
                                .execute(
                                    &bot,
                                    msg.chat.id,
                                    user_id,
                                    &args,
                                    state.engine.storage.clone(),
                                    state.rag.clone(),
                                    state.wasm.clone(),
                                    state.engine.allowed_paths.clone(),
                                    &state.db_pool,
                                )
                                .await
                            {
                                Ok(result) => {
                                    memory.add_message(MemoryMessage::tool_result(
                                        tool_call.id.clone(),
                                        result,
                                    ));
                                }
                                Err(e) => {
                                    tracing::error!("Tool execution error: {}", e);
                                    memory.add_message(MemoryMessage::tool_result(
                                        tool_call.id.clone(),
                                        format!("Error: {}", e),
                                    ));
                                }
                            }
                        } else {
                            memory.add_message(MemoryMessage::tool_result(
                                tool_call.id.clone(),
                                format!("Unknown tool: {}", tool_name),
                            ));
                        }
                    }
                    turn += 1;
                }
                Err(e) => {
                    tracing::error!("Engine error: {}", e);
                    bot.send_message(
                        msg.chat.id,
                        "Sorry, I ran into an error generating a response.",
                    )
                    .await?;
                    break;
                }
            }
        }
    }

    Ok(())
}

/// Handle /start command.
pub async fn handle_start(bot: Bot, msg: TgMessage, state: AppState) -> ResponseResult<()> {
    // Everyone can use /start in multi-user mode
    let welcome = state.i18n.get_message("en", "welcome", None);
    bot.send_message(msg.chat.id, welcome)
        .parse_mode(ParseMode::MarkdownV2)
        .await?;

    Ok(())
}

/// Handle /help command.
pub async fn handle_help(bot: Bot, msg: TgMessage, _state: AppState) -> ResponseResult<()> {
    // Everyone can use /help
    let help_text =
        "ClavaMea - I can search the web, read files, and remember things. Just talk to me!";
    bot.send_message(msg.chat.id, help_text)
        .parse_mode(ParseMode::MarkdownV2)
        .await?;

    Ok(())
}

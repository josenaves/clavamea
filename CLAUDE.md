# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Primary Reference

Always consult `CLAVAMEA_SPEC.md` for the definitive project specification, requirements, and architectural decisions.

## Core Principles

1. **Security First**: Never expose user data. Filter all messages by `OWNER_ID`.
2. **Privacy**: No logging of message contents to stdout. Use `tracing` for technical metadata only.
3. **Local-First**: All data stays in the local SQLite database.
4. **Multi-Language**: Support English and Portuguese (BR) via Fluent.

## Code Style

- **Error Handling**: Never use `unwrap()`. Use `anyhow::Result` and `thiserror` for custom errors.
- **State Management**: Use the `State` pattern with `Arc` for sharing resources.
- **Telegram Responses**: Use `MarkdownV2` for all bot responses.
- **Modularity**: Keep the module structure clean as defined in the spec.

## Architecture

The project follows these modules:

- `src/bot/`: Telegram handlers, routing, and state
- `src/core/`: LLM engine, memory, prompts, and tools
- `src/db/`: Database connections, migrations, models, and queries
- `src/i18n/`: Language detection, bundle management, and resource loading

### Key Files and Flow

- **Entry Point**: `src/main.rs` initializes the bot and starts the event loop.
- **Bot Initialization**: `src/bot/mod.rs` sets up the Telegram dispatcher and middleware.
- **Routing**: `src/bot/router.rs` defines message handlers and applies owner filtering.
- **LLM Engine**: `src/core/engine.rs` handles communication with the OpenAI-compatible API (DeepSeek).
- **Memory**: `src/core/memory.rs` manages conversation history stored in SQLite.
- **Database Layer**: 
  - Connection: `src/db/connection.rs`
  - Models: `src/db/models.rs`
  - Queries: `src/db/queries.rs`
  - Migrations: `src/db/migrations.rs`
- **Internationalization**: 
  - Detection: `src/i18n/detection.rs`
  - Loader: `src/i18n/loader.rs`
  - Bundle: `src/i18n/bundle.rs`
- **State Pattern**: Core resources (database connection, LLM engine) are wrapped in `Arc` and shared via a `State` struct.

## Development Workflow

1. Read the spec thoroughly before making changes.
2. Update documentation when adding features.
3. Local Validation: Run `cargo make ci` before pushing to verify fmt, clippy, and tests.
4. Verify owner filtering works correctly.

### Common Commands

- **Build**: `cargo build`
- **Run**: `cargo run` (requires `.env` with `OWNER_ID`, `TELEGRAM_BOT_TOKEN`, `LLM_API_KEY`, `LLM_API_BASE`)
- **Test Suite**: `cargo test`
- **Single Test**: `cargo test test_function_name`
- **Lint**: `cargo clippy`
- **Format**: `cargo fmt`
- **Check Format**: `cargo fmt -- --check`
- **CI Workflow**: `cargo make ci` (runs fmt, clippy, and tests)
- **Database Migration**: `sqlx migrate run` (requires `DATABASE_URL` in `.env`)
- **Reset Database**: `sqlx migrate reset` (use with caution)

## Environment Variables

Required environment variables are defined in `.env.example`. Never commit `.env` files.

Key variables:
- `OWNER_ID`: Telegram user ID of the bot owner (for message filtering)
- `TELEGRAM_BOT_TOKEN`: Token from @BotFather
- `LLM_API_KEY`: API key for OpenAI-compatible service (DeepSeek)
- `LLM_API_BASE`: Base URL for the LLM API
- `DATABASE_URL`: SQLite connection string (defaults to `sqlite://clavamea.db`)

## Important Notes

- All database queries must use parameterized statements via `sqlx` to prevent SQL injection.
- Telegram responses must escape special characters for MarkdownV2.
- The bot uses long polling; no webhook setup is required.
- i18n strings are stored in `locales/en.ftl` and `locales/pt-BR.ftl`.
# CLAUDE.md

This file contains instructions for AI agents working on the ClavaMea project.

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

## Development Workflow

1. Read the spec thoroughly before making changes.
2. Update documentation when adding features.
3. Local Validation: Run `cargo make ci` before pushing to verify fmt, clippy, and tests.
4. Verify owner filtering works correctly.

## Environment Variables

Required environment variables are defined in `.env.example`. Never commit `.env` files.
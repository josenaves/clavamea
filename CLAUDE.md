# CLAUDE.md — ClavaMea AI Agent Reference

This file contains instructions for AI agents working on the ClavaMea project.

---

## Primary Reference

Always consult `CLAVAMEA_SPEC.md` for the definitive project specification, requirements, and architectural decisions.

---

## Project Identity

- **Name**: ClavaMea ("My Club") — a sovereign, private AI assistant
- **Version**: from `CARGO_PKG_VERSION` (read more at `DOCS_VERSIONING.md`)
- **Language**: Rust (edition 2024, stable)
- **Deployment**: CasaOS via Docker (multi-stage build, `ghcr.io` registry)
- **Interface**: Telegram Bot (long polling) + WhatsApp (via separate bridge)
- **License**: MIT

---

## Core Principles

1. **Security First**: Never expose user data. Filter all messages by `OWNER_ID`.
2. **Privacy**: No logging of message contents to stdout. Use `tracing` for technical metadata only.
3. **Local-First**: All data stays in the local SQLite database.
4. **Multi-Language**: Support English and Portuguese (BR) via Fluent.

---

## Module Architecture

```
src/
├── main.rs           Entry point — initializes all subsystems
├── bot/              Telegram interface layer
│   ├── handlers.rs   Message handling, admin commands, LLM loop
│   ├── router.rs     Teloxide dispatcher routing
│   ├── state.rs      AppState (shared via Arc)
│   ├── scheduler.rs  Background scheduler (reminders, bovespa, web search)
│   └── utils.rs      Message chunking and retry logic
├── core/             AI/LLM orchestration
│   ├── engine.rs     LLM API client (NVIDIA → OpenRouter → DeepSeek)
│   ├── memory.rs     Conversation history management
│   ├── prompt.rs     System prompt builder (loads prompts/system.txt)
│   ├── tools.rs      30+ tool definitions and executors
│   ├── rag.rs        Local RAG (fastembed + SQLite vector search)
│   ├── renderer.rs   Telegram renderers (HTML + MarkdownV2)
│   ├── router.rs     OpenRouter multi-model router
│   ├── storage.rs    File-based memory (SOUL/USER/MEMORY.md per user)
│   ├── wasm.rs       WebAssembly sandbox (wasmtime)
│   └── genetics.rs   Hardy-Weinberg & Punnett square calculator
├── db/               Database layer
│   ├── connection.rs SQLite connection pool creation
│   ├── migrations.rs Migration runner
│   ├── models.rs     Data model structs
│   └── queries.rs    All CRUD operations (~934 lines)
├── i18n/             Internationalization
│   ├── bundle.rs     Fluent bundle manager
│   ├── detection.rs  Language detection from Telegram metadata
│   └── loader.rs     FTL file loader
└── whatsapp/         WhatsApp integration
    ├── sender.rs     Bridge HTTP client
    └── webhook.rs    Axum webhook server
```

---

## Code Style Rules

- **Error Handling**: Never use `unwrap()`. Use `anyhow::Result` and `thiserror` for custom errors.
- **State Management**: Use the `State` pattern with `Arc` for sharing resources.
- **Telegram Responses**: Use `MarkdownV2` for all bot responses.
- **Modularity**: Keep the module structure clean as defined in the spec.
- **Testing**: Tests are `#[cfg(test)] mod tests` inline — run single-threaded (`--test-threads=1`).
- **No comments in code** unless there's a specific reason.
- **Follow existing patterns** — check neighboring files before writing new code.

---

## Key Architecture Patterns

### AppState (`src/bot/state.rs`)
Shared via `Arc<AppState>`, contains:
- `db_pool` — SQLite pool
- `engine` — LLM engine
- `i18n` — Bundles
- `rag` — Vector search
- `wasm` — Wasm runtime
- `owner_id`, `max_conversation_length`
- `bot` — teloxide::Bot
- `user_locks` — DashMap for per-user mutexes
- `processed_messages` — DashSet for dedup

### LLM Provider Chain (`src/main.rs:203-426`)
Priority: `LLM_PROVIDER` env var → auto-detect
- NVIDIA (if configured) → OpenRouter → DeepSeek → Placeholder
- Fallback: if primary fails, try next provider
- Tiered routing: Pro model (turn 0), Flash model (follow-ups)

### Conversation Loop (`src/bot/handlers.rs`)
- Owner filter → authorization check → up to 20 turns
- Tool execution → LLM response → repeat (tools loop)
- Admin commands: `/changelog`, `/users`, `/approve`, `/deauth`

### Tool System (`src/core/tools.rs`)
- 30+ tools in OpenAI function calling format
- Gated by phase (currently all phase 3)
- Admin-only: `UpdateServer`
- Path sandboxing for file operations

### Security Architecture
- **Owner lock**: Every message checked against `OWNER_ID`
- **Authorization**: Users start `pending`, must be approved
- **Roles**: owner > admin > family > friend > subscriber > pending
- **Path sandbox**: Validates paths against project root + `ALLOWED_ORGANIZE_PATHS`
- **URL safety**: `is_safe_url()` blocks localhost, private IPs, metadata endpoints
- **Docker isolation**: Containerized deployment

---

## Database Schema (11 migrations)

| Table | Key Columns | Migration |
|---|---|---|
| `interactions` | chat_id, role, content, lang | 0001 |
| `documents` | user_id, filename, path | 0002 |
| `document_chunks` | document_id (FK), content, embedding (BLOB) | 0002 |
| `vehicles` | user_id, name, model, plate | 0003 |
| `fuel_logs` | vehicle_id (FK), odometer, liters, price_per_liter | 0003 |
| `expense_logs` | vehicle_id (FK), category, cost | 0003 |
| `users` | id (PK = Telegram ID), username, role, authorized, timezone | 0005-0010 |
| `schedules` | user_id (FK), cron_expr, task_type, search_query | 0007-0011 |
| `book_episodes` | user_id (FK), content, tags, phase | 0009 |
| `book_chapters` | user_id (FK), order_num, title, filepath | 0009 |

---

## Environment Variables

| Variable | Required | Default | Notes |
|---|---|---|---|
| `TELEGRAM_BOT_TOKEN` | **Yes** | — | From @BotFather |
| `OWNER_ID` | **Yes** | — | Telegram numeric user ID |
| `DATABASE_URL` | No | `sqlite:clavamea.db` | SQLite URL |
| `LLM_PROVIDER` | No | `auto` | `auto`/`nvidia`/`openrouter`/`deepseek` |
| `LLM_API_URL` | Conditional | — | DeepSeek/OpenRouter URL |
| `LLM_API_KEY` | Conditional | — | DeepSeek/OpenRouter key |
| `LLM_MODEL` / `_PRO` / `_FLASH` | No | `deepseek-chat` | Model names |
| `NVIDIA_API_URL` / `_KEY` / `_MODEL_PRO` / `_MODEL_FLASH` | Conditional | — | NVIDIA config |
| `OPENROUTER_API_KEY` / `_MODELS` / `_TIMEOUT` | Conditional | — | OpenRouter config |
| `BRAVE_API_KEY` | Conditional | — | Web search tool |
| `GITHUB_TOKEN` | Conditional | — | GitHub tools |
| `SERVER_UPDATE_PATH` | Conditional | — | Self-update path |
| `MEMORY_DIR` | No | `./memory` | Per-user file memory |
| `LOCALES_DIR` | No | `./locales` | Fluent .ftl files |
| `MAX_CONVERSATION_LENGTH` | No | `20` | History length |
| `LOG_LEVEL` | No | `info` | Tracing level |
| `ALLOWED_ORGANIZE_PATHS` | No | — | File ops sandbox |
| `DISABLE_PATH_SANDBOX` | No | — | Disable path restrictions |
| `WHATSAPP_BRIDGE_URL` | No | — | Bridge service URL |
| `WHATSAPP_WEBHOOK_PORT` | No | `8081` | Webhook port |

---

## Development Workflow

1. Read the spec and this file thoroughly before making changes.
2. Update documentation when adding features.
3. Local Validation: Run `cargo make ci` before pushing.
4. Verify owner filtering works correctly.
5. Tests run single-threaded: `cargo test -- --test-threads=1`.

### `cargo make` tasks:
- `ci` — fmt + clippy + build + test (full pipeline)
- `fmt-check` — Rustfmt check
- `clippy-check` — Clippy with `-D warnings`
- `build-debug` — Debug build
- `test-project` — All tests (single-threaded)
- `prepare` — Cache SQLx queries for offline compilation

### CI Pipeline (`ci.yml`):
Triggers on push/PR to `main`: fmt → clippy → build → test (`SQLX_OFFLINE=true`)

### Docker Publish (`docker-publish.yml`):
Triggers on `main` pushes and `v*.*.*` tags → builds multi-platform and pushes to `ghcr.io`

---

## Key File Reference

| File | Lines | Purpose |
|---|---|---|
| `src/main.rs` | ~504 | Entry point, init all subsystems |
| `src/bot/handlers.rs` | ~537 | Core message handler + admin commands |
| `src/bot/scheduler.rs` | ~473 | Background task scheduler |
| `src/bot/state.rs` | ~64 | Shared AppState |
| `src/bot/utils.rs` | ~175 | Message sending utilities |
| `src/core/engine.rs` | ~494 | LLM API client |
| `src/core/tools.rs` | ~2280 | All tool definitions + executors |
| `src/core/rag.rs` | ~219 | Local RAG |
| `src/core/memory.rs` | ~141 | Conversation memory |
| `src/core/storage.rs` | ~295 | File-based memory |
| `src/core/renderer.rs` | ~321 | Markdown renderers |
| `src/core/wasm.rs` | ~115 | Wasm runtime |
| `src/core/genetics.rs` | ~244 | Genetics calculator |
| `src/core/router.rs` | ~152 | OpenRouter router |
| `src/core/prompt.rs` | ~31 | System prompt builder |
| `src/db/queries.rs` | ~934 | All CRUD queries |
| `src/db/models.rs` | ~232 | Database model structs |
| `src/i18n/bundle.rs` | ~71 | Fluent bundle manager |
| `src/whatsapp/webhook.rs` | ~362 | WhatsApp webhook server |
| `src/whatsapp/sender.rs` | ~74 | Bridge HTTP client |

---

## Important Gotchas

- **No `unwrap()`** ever — use `anyhow::Context` or proper error handling.
- **SQLX_OFFLINE=true** is needed during Docker builds (`.sqlx/` caches query metadata).
- **DashMap/DashSet** for concurrent thread-safe maps (user locks, message dedup).
- **Fluent i18n** — translations in `locales/en.ftl` and `locales/pt-BR.ftl`.
- **Book/vehicle data** is auto-ingested into RAG at startup (`src/main.rs`).
- **WhatsApp is a separate service** (`whatsapp-bridge/`) — not compiled into main binary.
- **`is_safe_url()`** blocks dangerous URLs (localhost, private IPs, metadata endpoints).
- **LLM conversation loop** cycles: tool call → execute → result → next LLM call (max 20 turns).
- **Schedule cron format**: `HH:MM DAY` (recurring, e.g. `17:10 MON-FRI`) or `YYYY-MM-DD HH:MM` (one-time).
- **Changelog** is a constant string in `handlers.rs`, manually maintained in Portuguese.

# 🧠 ClavaMea

**Sovereign, private AI assistant** with Telegram interface.

> **ClavaMea** (Latin for "My Club") is a local-first AI assistant that gives you absolute control over your data and privacy.

## Features

- 🔒 **Owner-only access**: Filtered by `OWNER_ID` environment variable
- 💾 **Local storage**: All conversations stored in SQLite database
- 🌐 **Multi-language**: English and Portuguese (BR) support via Fluent
- 🤖 **Telegram bot**: Long polling (no open ports required)
- 🧠 **LLM integration**: OpenAI-compatible API (DeepSeek)
- 🛠️ **Extensible**: Prepared for tool calling and skills

## Tech Stack

- **Language**: Rust (stable)
- **Async runtime**: Tokio
- **Telegram**: teloxide
- **Database**: sqlx + SQLite
- **i18n**: fluent-rs (Project Fluent)
- **Error handling**: anyhow + thiserror
- **Logging**: tracing

## Getting Started

### Prerequisites

- Rust (stable) and Cargo
- Telegram Bot Token from [@BotFather](https://t.me/botfather)
- Your Telegram User ID from [@userinfobot](https://t.me/userinfobot)
- DeepSeek API key (or other OpenAI-compatible API)

### Installation

1. Clone the repository:
   ```bash
   git clone <repository-url>
   cd clavamea
   ```

2. Copy environment variables:
   ```bash
   cp .env.example .env
   ```

3. Edit `.env` with your configuration.

4. Build and run:
   ```bash
   cargo build --release
   cargo run --release
   ```

## Project Structure

```
src/
├── bot/          # Telegram handlers and routing
├── core/         # LLM orchestration and prompt building
├── db/           # Database schemas and queries
└── i18n/         # Internationalization logic
locales/          # Fluent translation files (.ftl)
prompts/          # System prompt templates
migrations/       # SQL migration files
```

## Development Roadmap

### Phase 1 (MVP)
- Direct chat with conversation memory
- Basic command handling

### Phase 2 (Tools)
- Web search integration
- CasaOS status monitoring
- File reader

### Phase 3 (Skills)
- Document RAG with local vector database
- Code interpreter (Wasm)

## License

MIT OR Apache-2.0
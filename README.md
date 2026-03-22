# 🧠 ClavaMea

**A Sovereign, Privacy-First AI Assistant for Telegram**

ClavaMea (Latin for *"My Club"*) is a high-performance, local-first AI assistant built in Rust. It is designed to provide a secure, private, and highly extensible interface for interacting with Large Language Models (LLMs) while keeping your data under your absolute control.

[![Rust CI](https://github.com/josenaves/clavamea/actions/workflows/ci.yml/badge.svg)](https://github.com/josenaves/clavamea/actions/workflows/ci.yml)
[![Docker Publish](https://github.com/josenaves/clavamea/actions/workflows/docker-publish.yml/badge.svg)](https://github.com/josenaves/clavamea/actions/workflows/docker-publish.yml)

---

## 🚀 Key Features

- 🔒 **Privacy by Design**: Local-first architecture with all conversation history and metadata stored in a local SQLite database.
- 👥 **Multi-User & Multi-Role**: Built-in access control system with user isolation. New users must be approved by the admin.
- 🧠 **Retrieval-Augmented Generation (RAG)**: Integration with a local vector database for semantic search and document indexing.
- 🏗️ **Wasm Code Execution**: A secure environment to execute code snippets safely using a WebAssembly runtime.
- 💬 **Premium Interface**: Rich Telegram interaction with MarkdownV2 support, localized responses (English/Portuguese), and interactive routing.
- 📦 **Docker & CasaOS Ready**: Optimized for containerized deployment on home servers like CasaOS.
- 🤖 **OpenAI-Compatible**: Works with any OpenAI-compatible API (e.g., DeepSeek, Local LLMs via Ollama/LocalAI).

---

## 🛠️ Tech Stack

- **Core**: [Rust](https://www.rust-lang.org/) (Stable)
- **Runtime**: [Tokio](https://tokio.rs/)
- **Telegram Bot API**: [Teloxide](https://teloxide.tshakalekholoane.dev/)
- **Database**: [SQLx](https://github.com/launchbadge/sqlx) (SQLite) + Vector Search
- **Wasm Runtime**: [Wasmtime](https://wasmtime.dev/)
- **i18n**: [Project Fluent](https://projectfluent.org/)
- **CI/CD**: GitHub Actions

---

## 🚥 Getting Started

### Prerequisites

- Rust (latest stable)
- A Telegram Bot Token from [@BotFather](https://t.me/botfather)
- An OpenAI-compatible API key (e.g., [DeepSeek](https://www.deepseek.com/))
- (Optional) [Docker](https://www.docker.com/) for containerized deployment

### Local Setup

1. **Clone the repository**:
   ```bash
   git clone https://github.com/josenaves/clavamea.git
   cd clavamea
   ```

2. **Configure Environment**:
   ```bash
   cp .env.example .env
   # Edit .env with your TELEGRAM_BOT_TOKEN and OWNER_ID
   ```

3. **Run the application**:
   ```bash
   cargo run --release
   ```

### Docker Deployment

```bash
docker pull ghcr.io/josenaves/clavamea:main
docker run -d \
  --name clavamea \
  -v ./data:/app/data \
  -v ./memory:/app/memory \
  --env-file .env \
  ghcr.io/josenaves/clavamea:main
```

---

## 🗺️ Roadmap

- [x] Multi-user isolation and approval system
- [x] Local RAG implementation (Vector DB)
- [x] Secure Wasm Code Interpreter
- [x] GitHub Actions CI/CD Pipeline
- [ ] Advanced Memory (Long-term semantic memory)
- [ ] Integration with Home Assistant
- [ ] Image generation and multimodal support

---

## ⚖️ License

Distributed under the **MIT License**. See `LICENSE` for more information.

---

Built with ❤️ by [José Naves](https://github.com/josenaves)
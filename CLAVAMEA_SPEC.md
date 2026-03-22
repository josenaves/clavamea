# 🧠 PROJECT SPECIFICATION: CLAVAMEA

## 1. IDENTITY & GOAL

**ClavaMea** (Latin for "My Club") is a sovereign, private AI assistant.

* **Core Philosophy:** Absolute user control, local-first data, and high-performance Rust execution.
* **Primary Environment:** CasaOS (running via Docker).
* **Interface:** Telegram Bot via Long Polling (no open ports required).

## 2. TECH STACK (STRICT)

* **Language:** Rust (Stable).
* **Async Runtime:** `tokio`.
* **Telegram:** `teloxide` (using the Dispatcher pattern).
* **Database:** `sqlx` + `sqlite` (local persistence).
* **i18n:** `fluent-rs` (Project Fluent).
* **LLM Engine:** OpenAI-compatible API (configured for DeepSeek).
* **Error Handling:** `anyhow` + `thiserror`.

## 3. ARCHITECTURAL REQUIREMENTS

### A. Security First

1. **The Owner Lock:** Every incoming message MUST be filtered by `OWNER_ID` (env var). If the sender ID doesn't match, drop the message.
2. **Data Sovereignty:** All conversation history must be stored in a local SQLite database (`clavamea.db`).
3. **Privacy:** No logging of message contents to `stdout`. Use `tracing` for technical metadata only.

### B. Modular Structure

* `src/bot/`: Telegram handlers and message routing.
* `src/core/`: LLM orchestration, prompt building, and tool calling logic.
* `src/db/`: Database schemas, migrations, and CRUD operations.
* `src/i18n/`: Translation loading and language detection logic.
* `locales/`: `.ftl` files for English and Portuguese (BR).

### C. Multi-Language (i18n)

* The system must detect the user's language from Telegram metadata (`language_code`).
* Static responses (e.g., "Thinking...", "Error") must use Fluent files.
* The system prompt for the LLM must be dynamically injected with the detected language.

## 4. DATABASE SCHEMA (MVP)

```sql
CREATE TABLE interactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chat_id INTEGER NOT NULL,
    role TEXT CHECK(role IN ('user', 'assistant', 'system')) NOT NULL,
    content TEXT NOT NULL,
    lang TEXT DEFAULT 'en',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

```

## 5. THE "TOOL & SKILLS" ROADMAP

The code must be prepared for **Function Calling**.

* **Phase 1 (MVP):** Direct chat with memory.
* **Phase 2 (Tools):** `web_search`, `file_reader`.
* **Phase 3 (Skills):** Document RAG (Local Vector DB), Code Interpreter (Wasm).

## 6. INSTRUCTIONS FOR THE AI AGENT (CLAUDE/DEEPSEEK)

1. **Safety:** Never use `unwrap()`. Handle all `Result` types gracefully.
2. **Patterns:** Use the `State` pattern (Atomic Reference Counting - `Arc`) to share the database and engine between threads.
3. **Telegram:** Use `MarkdownV2` for all bot responses.
4. **Consistency:** All module declarations (`mod.rs`) must be clean and documented.

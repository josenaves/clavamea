# 🚀 Resumption Guide: ClavaMea Project

This document summarises the current state of the project to facilitate resumption after a break.

## 📍 Current Status (v1.5.0)

### 1. Core Features (Status: OK)
- **Telegram Bot**: Operational, backed by SQLite persistence.
- **Memory**: File-based system (`SOUL.md`, `USER.md`, `MEMORY.md`) fully integrated.
- **RAG (Document Search)**: `fastembed` engine is functional. Can index and semantically search local documents.
- **Multi-user**: Role-based access control (owner, admin, family, friend). Real names stored per user.

### 2. Code Interpreter (Status: OK — WAT)
- **Runtime**: Built on `wasmtime` 29.0.
- **Isolation**: Secure sandbox via WASI.
- **Capability**: Executes WebAssembly Text (WAT) with `stdout` capture.

### 3. Scheduler (Status: OK — bugs fixed in v1.5.0 ✅)
- **Loop**: Checks the database every 60 seconds for due tasks.
- **Task types**: `reminder` (one-time and recurring), `bovespa_clipping`.
- **Supported formats**:
  - `HH:MM MON-FRI` — recurring on weekdays
  - `HH:MM DAY` — recurring on a specific day (e.g. `09:00 MON`)
  - `YYYY-MM-DD HH:MM` — one-time (fires once, then self-deletes)
- **Bug fixed (v1.5.0)**: `today` now uses `Local::now()` instead of `Utc::now()` — reminders no longer silently fail in negative UTC offsets (e.g. UTC-3 after 21:00).
- **Bug fixed (v1.5.0)**: `is_one_time` now correctly detects `YYYY-MM-DD` expressions instead of using `.contains('-')`, which was deleting `MON-FRI` reminders after their first execution.

### 4. Docker & Deployment (Status: OK)
- **Dockerfile**: Ready (multi-stage build, ~85 MB).
- **Docker Compose**: Configured for CasaOS with volumes for `/data` (DB) and `/memory`.

---

## 🛠️ Deployment Quick Reference

### Step 1: Build the Image
```bash
docker build -t your-user/clavamea:latest .
```

### Step 2: Push to Registry
```bash
docker login
docker push your-user/clavamea:latest
```

### Step 3: Deploy on CasaOS (Ubuntu Server)
1. Create a directory `/home/user/clavamea`.
2. Place your `.env` file there.
3. Drop the `docker-compose.yml` (already in the project).
4. Run:
```bash
docker-compose up -d
```

---

## 📅 Next Steps (Suggestions)

- [ ] **Reminder management tools**: Add `list_reminders` and `cancel_reminder` so the LLM can list and cancel scheduled tasks directly from the chat.
- [ ] **Extended recurrence patterns**: Support "every day" (`HH:MM *`), "every month on day X", etc.
- [ ] **Wasm fuel limits**: Implement CPU/memory caps (fuel metering) in Wasmtime.
- [ ] **JS Runtime**: Experiment with embedding `QuickJS` compiled to Wasm for a JavaScript code interpreter.
- [ ] **Field testing**: Send real documents to the bot and index them via `index_document`.

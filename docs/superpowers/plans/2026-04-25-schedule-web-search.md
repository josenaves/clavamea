# Schedule Web Search Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Nova tool `schedule_web_search` que agenda lembretes recorrentes que fazem busca na internet automaticamente quando disparam.

**Architecture:** Nova tool em `tools.rs` + coluna `search_query` na DB + handler no scheduler para executar busca.

**Tech Stack:** Rust, SQLx, Telegram Bot API

---

## Task 1: Add search_query column to database

**Files:**
- Modify: `src/db/queries.rs` (migration)
- Modify: `src/db/models.rs` (Schedule struct)

### Step 1: Add column to Schedule model

- [ ] **Step 1: Modify Schedule struct**

Modify: `src/db/models.rs:137-147`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Schedule {
    pub id: i64,
    pub user_id: i64,
    pub cron_expr: String,
    pub task_type: String,
    pub payload: Option<String>,
    pub last_run: Option<String>,
    pub created_at: DateTime<Utc>,
    pub search_query: Option<String>,  // ADD THIS
}
```

- [ ] **Step 2: Run tests**

Run: `cargo check`
Expected: Compiles

- [ ] **Step 3: Commit**

```bash
git add src/db/models.rs
git commit -m "feat(db): add search_query column to Schedule model"
```

---

## Task 2: Add schedule_web_search tool

**Files:**
- Modify: `src/core/tools.rs`

### Step 1: Add tool definition

- [ ] **Step 1: Add schedule_web_search tool**

Add after schedule_reminder tool definition in `src/core/tools.rs`:

```rust
Tool::ScheduleWebSearch => serde_json::json!({
    "name": "schedule_web_search",
    "description": "Schedule a recurring reminder that performs a web search when triggered. Use for periodic information updates like sports scores, news, etc.",
    "parameters": {
        "type": "object",
        "properties": {
            "message": {
                "type": "string",
                "description": "Confirmation message to show when scheduled (e.g., 'Te aviso toda segunda 8:00')"
            },
            "cron_expr": {
                "type": "string", 
                "description": "Cron expression: 'HH:MM DAY' (e.g., '08:00 MON' for every monday 8am, '08:00 MON-FRI' for weekdays)"
            },
            "search_query": {
                "type": "string",
                "description": "What to search for (e.g., 'resultados jogos Cruzeiro', 'notícias do bitcoin')"
            }
        },
        "required": ["message", "search_query"]
    }
})
```

- [ ] **Step 2: Add tool executor**

Add new enum variant and execution logic:

```rust
Tool::ScheduleWebSearch => {
    let args: ScheduleReminderArgs = serde_json::from_value(args)?;
    // Insert into schedules table with task_type = "web_search"
    let cron_expr = format!("{} {}", args.time, args.days);
    let search_query = args.search_query;
    
    sqlx::query(
        "INSERT INTO schedules (user_id, cron_expr, task_type, payload, search_query) VALUES (?, ?, 'web_search', ?, ?)"
    )
    .bind(user_id)
    .bind(cron_expr)
    .bind(message)
    .bind(search_query)
    .execute(&pool)
    .await?;
    
    Ok(serde_json::json!({"ok": true, "message": message}))
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test tools -- --test-threads=1`
Expected: Tests pass

- [ ] **Step 4: Commit**

```bash
git add src/core/tools.rs
git commit -m "feat(tools): add schedule_web_search tool"
```

---

## Task 3: Handle web_search in scheduler

**Files:**
- Modify: `src/bot/scheduler.rs`

### Step 1: Add web_search handler

- [ ] **Step 1: Add web_search case in process_due_tasks**

Modify: `src/bot/scheduler.rs` around line 78, add new case:

```rust
"web_search" => {
    let state_clone = state.clone();
    let user_id = task.user_id;
    let search_query = task.search_query.clone().unwrap_or_default();
    let payload = task.payload.clone();
    let schedule_id = task.id;
    let is_one_time = is_one_time_expr(&task.cron_expr);

    tokio::spawn(async move {
        if let Err(e) = execute_web_search(
            state_clone,
            user_id,
            search_query,
            payload,
            schedule_id,
            is_one_time,
        )
        .await
        {
            error!("Web search failed for user {}: {}", user_id, e);
        }
    });
}
```

- [ ] **Step 2: Implement execute_web_search function**

Add after execute_reminder function:

```rust
async fn execute_web_search(
    state: AppState,
    user_id: i64,
    search_query: String,
    message: Option<String>,
    schedule_id: i64,
    is_one_time: bool,
) -> anyhow::Result<()> {
    info!("Running web search for user {}: {}", user_id, search_query);

    // Use web_search tool to get results
    let tools = vec![crate::core::tools::Tool::WebSearch];

    let memory = crate::core::memory::ConversationMemory::new(user_id, 1);
    memory.add_message(crate::core::memory::Message {
        role: Role::User,
        content: search_query.clone(),
    });

    let response = state
        .engine
        .generate(user_id, &memory, &tools, "en", None, None)
        .await?;

    let text = match response {
        crate::core::engine::LLMResponse::Text(t) => t,
        crate::core::engine::LLMResponse::ToolCalls(_) => search_query,
    };

    // Send to user
    let state_clone = state.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::bot::utils::send_message(state_clone, user_id, &text).await {
            error!("Failed to send web search result: {}", e);
        }
    });

    // Update last_run
    if is_one_time {
        sqlx::query("DELETE FROM schedules WHERE id = ?")
            .bind(schedule_id)
            .execute(&state.db_pool)
            .await?;
    } else {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE schedules SET last_run = ? WHERE id = ?")
            .bind(now)
            .bind(schedule_id)
            .execute(&state.db_pool)
            .await?;
    }

    Ok(())
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -- --test-threads=1`
Expected: Tests pass

- [ ] **Step 4: Commit**

```bash
git add src/bot/scheduler.rs
git commit -m "feat(scheduler): add web_search task handler"
```

---

## Task 4: Update system prompt

**Files:**
- Modify: `src/core/engine.rs`

### Step 1: Add tool to system prompt

- [ ] **Step 1: Update system prompt**

Modify the system prompt in engine.rs around line 84 to include schedule_web_search:

```rust
- schedule_reminder: for reminders, notifications, callbacks\n\
+ schedule_reminder: for simple reminders without internet search\n\
+ schedule_web_search: for recurring reminders that search the web when triggered (sports scores, news, etc.)\n\
```

- [ ] **Step 2: Commit**

```bash
git commit -m "feat(engine): add schedule_web_search to system prompt"
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Add search_query column | models.rs |
| 2 | Add tool definition | tools.rs |
| 3 | Add scheduler handler | scheduler.rs |
| 4 | Update system prompt | engine.rs |

Total esperado: ~4 commits
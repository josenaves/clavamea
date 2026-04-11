//! Database queries and CRUD operations.

use anyhow::Result;
use chrono::{DateTime, Local, Utc};

use crate::db::connection::Pool;
use crate::db::models::{ExpenseLog, FuelLog, Interaction, NewInteraction, User, Vehicle};

/// Insert a new interaction into the database.
pub async fn insert_interaction(pool: &Pool, interaction: &NewInteraction) -> Result<i64> {
    let result = sqlx::query(
        r#"
        INSERT INTO interactions (chat_id, role, content, lang)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(interaction.chat_id)
    .bind(&interaction.role)
    .bind(&interaction.content)
    .bind(&interaction.lang)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

/// Get the last N interactions for a chat, ordered by creation date.
pub async fn get_recent_interactions(
    pool: &Pool,
    chat_id: i64,
    limit: u32,
) -> Result<Vec<Interaction>> {
    let interactions = sqlx::query_as::<_, Interaction>(
        r#"
        SELECT id, chat_id, role, content, lang, created_at
        FROM interactions
        WHERE chat_id = ?
        ORDER BY created_at DESC
        LIMIT ?
        "#,
    )
    .bind(chat_id)
    .bind(limit as i64)
    .fetch_all(pool)
    .await?;

    // Reverse to get chronological order
    Ok(interactions.into_iter().rev().collect())
}

/// Get all interactions for a chat within a time range.
pub async fn get_interactions_in_range(
    pool: &Pool,
    chat_id: i64,
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<Interaction>> {
    let interactions = sqlx::query_as::<_, Interaction>(
        r#"
        SELECT id, chat_id, role, content, lang, created_at
        FROM interactions
        WHERE chat_id = ? AND created_at BETWEEN ? AND ?
        ORDER BY created_at ASC
        "#,
    )
    .bind(chat_id)
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await?;

    Ok(interactions)
}

/// Delete all interactions for a chat (cleanup).
pub async fn delete_chat_interactions(pool: &Pool, chat_id: i64) -> Result<u64> {
    let result = sqlx::query("DELETE FROM interactions WHERE chat_id = ?")
        .bind(chat_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

/// Insert a new vehicle.
pub async fn insert_vehicle(
    pool: &Pool,
    user_id: i64,
    name: &str,
    model: Option<&str>,
    plate: Option<&str>,
) -> Result<i64> {
    let result =
        sqlx::query("INSERT INTO vehicles (user_id, name, model, plate) VALUES (?, ?, ?, ?)")
            .bind(user_id)
            .bind(name)
            .bind(model)
            .bind(plate)
            .execute(pool)
            .await?;

    Ok(result.last_insert_rowid())
}

/// List all vehicles for a user.
pub async fn get_vehicles(pool: &Pool, user_id: i64) -> Result<Vec<Vehicle>> {
    let vehicles =
        sqlx::query_as::<_, Vehicle>("SELECT * FROM vehicles WHERE user_id = ? ORDER BY name ASC")
            .bind(user_id)
            .fetch_all(pool)
            .await?;
    Ok(vehicles)
}

/// Verify if a vehicle belongs to a user.
pub async fn is_vehicle_owner(pool: &Pool, vehicle_id: i64, user_id: i64) -> Result<bool> {
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM vehicles WHERE id = ? AND user_id = ?")
            .bind(vehicle_id)
            .bind(user_id)
            .fetch_one(pool)
            .await?;
    Ok(count.0 > 0)
}

/// Insert a fuel log.
pub async fn insert_fuel_log(
    pool: &Pool,
    vehicle_id: i64,
    odometer: f64,
    liters: f64,
    price_per_liter: f64,
    fuel_type: &str,
    total_cost: f64,
) -> Result<i64> {
    let result = sqlx::query(
        r#"
        INSERT INTO fuel_logs (vehicle_id, odometer, liters, price_per_liter, fuel_type, total_cost)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(vehicle_id)
    .bind(odometer)
    .bind(liters)
    .bind(price_per_liter)
    .bind(fuel_type)
    .bind(total_cost)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

/// Get the last fuel log for a vehicle.
pub async fn get_last_fuel_log(pool: &Pool, vehicle_id: i64) -> Result<Option<FuelLog>> {
    let log = sqlx::query_as::<_, FuelLog>(
        "SELECT * FROM fuel_logs WHERE vehicle_id = ? ORDER BY odometer DESC LIMIT 1",
    )
    .bind(vehicle_id)
    .fetch_optional(pool)
    .await?;
    Ok(log)
}

/// Insert an expense log.
pub async fn insert_expense_log(
    pool: &Pool,
    vehicle_id: i64,
    category: &str,
    description: Option<&str>,
    cost: f64,
) -> Result<i64> {
    let result = sqlx::query(
        "INSERT INTO expense_logs (vehicle_id, category, description, cost) VALUES (?, ?, ?, ?)",
    )
    .bind(vehicle_id)
    .bind(category)
    .bind(description)
    .bind(cost)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

/// Get fuel logs for a vehicle in a date range.
pub async fn get_vehicle_fuel_logs(
    pool: &Pool,
    vehicle_id: i64,
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<FuelLog>> {
    let logs = sqlx::query_as::<_, FuelLog>(
        "SELECT * FROM fuel_logs WHERE vehicle_id = ? AND date BETWEEN ? AND ? ORDER BY date ASC",
    )
    .bind(vehicle_id)
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await?;
    Ok(logs)
}

/// Get expense logs for a vehicle in a date range.
pub async fn get_vehicle_expenses(
    pool: &Pool,
    vehicle_id: i64,
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<ExpenseLog>> {
    let logs = sqlx::query_as::<_, ExpenseLog>(
        "SELECT * FROM expense_logs WHERE vehicle_id = ? AND date BETWEEN ? AND ? ORDER BY date ASC"
    )
    .bind(vehicle_id)
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await?;
    Ok(logs)
}

// --- USER QUERIES ---

/// Get a user by ID.
pub async fn get_user(pool: &Pool, user_id: i64) -> Result<Option<User>> {
    let user = sqlx::query_as::<_, User>(
        "SELECT id, username, role, authorized, last_seen_version, full_name, created_at FROM users WHERE id = ?"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(user)
}

/// Create a new pending user.
pub async fn create_user_pending(pool: &Pool, user_id: i64, username: Option<&str>) -> Result<()> {
    sqlx::query(
        "INSERT OR IGNORE INTO users (id, username, role, authorized) VALUES (?, ?, 'pending', 0)",
    )
    .bind(user_id)
    .bind(username)
    .execute(pool)
    .await?;
    Ok(())
}

/// Ensure the owner exists and is authorized.
pub async fn ensure_owner_exists(pool: &Pool, owner_id: i64) -> Result<()> {
    // Always ensure the owner is set as owner, regardless of current state
    sqlx::query(
        "INSERT OR REPLACE INTO users (id, role, authorized, username, full_name) 
         VALUES (?, 'owner', 1, 
                 COALESCE((SELECT username FROM users WHERE id = ?), NULL),
                 COALESCE((SELECT full_name FROM users WHERE id = ?), NULL))",
    )
    .bind(owner_id)
    .bind(owner_id)
    .bind(owner_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Authorize a user with a specific role and name.
pub async fn authorize_user_with_name(
    pool: &Pool,
    user_id: i64,
    role: &str,
    full_name: Option<&str>,
) -> Result<()> {
    sqlx::query("UPDATE users SET role = ?, authorized = 1, full_name = ? WHERE id = ?")
        .bind(role)
        .bind(full_name)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Revoke user authorization.
pub async fn deauthorize_user(pool: &Pool, user_id: i64) -> Result<()> {
    sqlx::query("UPDATE users SET authorized = 0 WHERE id = ?")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// List all users.
pub async fn list_users(pool: &Pool) -> Result<Vec<User>> {
    let users = sqlx::query_as::<_, User>(
        "SELECT id, username, role, authorized, last_seen_version, full_name, created_at FROM users ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await?;
    Ok(users)
}

/// Update the last seen bot version for a user.
pub async fn update_user_seen_version(pool: &Pool, user_id: i64, version: &str) -> Result<()> {
    sqlx::query("UPDATE users SET last_seen_version = ? WHERE id = ?")
        .bind(version)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

// --- SCHEDULE QUERIES ---

/// Fetch schedules that are due to run at the given time and weekday.
pub async fn get_due_schedules(
    pool: &Pool,
    time_str: &str,
    weekday: &str,
) -> Result<Vec<crate::db::models::Schedule>> {
    let all_schedules = sqlx::query_as::<_, crate::db::models::Schedule>("SELECT * FROM schedules")
        .fetch_all(pool)
        .await?;

    let mut due = Vec::new();
    // Use Local time so reminders match the user's timezone (not UTC)
    let today = Local::now().format("%Y-%m-%d").to_string();

    for s in all_schedules {
        let parts: Vec<&str> = s.cron_expr.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        // Logic branching: if parts[0] is a date (YYYY-MM-DD), it's a one-time event
        if parts[0].len() == 10 && parts[0].contains('-') {
            if parts.len() < 2 {
                continue;
            } // malformed
            let target_date = parts[0];
            let target_time = parts[1];

            if target_date == today && target_time == time_str {
                due.push(s);
            }
            continue;
        }

        // Otherwise, it's a recurring event starting with HH:MM
        let target_time = parts[0];
        if target_time != time_str {
            continue;
        }

        // Check if already run today to avoid double triggers in the same minute
        if let Some(last) = &s.last_run {
            if last.starts_with(&today) {
                continue;
            }
        }

        // Check weekday if specified
        if parts.len() > 1 {
            let days = parts[1];
            if days == "MON-FRI" {
                if weekday == "SAT" || weekday == "SUN" {
                    continue;
                }
            } else if !days.contains(weekday) {
                continue;
            }
        }

        due.push(s);
    }

    Ok(due)
}

/// Update the last run timestamp for a schedule.
pub async fn update_schedule_last_run(pool: &Pool, schedule_id: i64) -> Result<()> {
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    sqlx::query("UPDATE schedules SET last_run = ? WHERE id = ?")
        .bind(now)
        .bind(schedule_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Delete a schedule.
pub async fn delete_schedule(pool: &Pool, schedule_id: i64) -> Result<()> {
    sqlx::query("DELETE FROM schedules WHERE id = ?")
        .bind(schedule_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Insert a new schedule task.
pub async fn insert_schedule(
    pool: &Pool,
    user_id: i64,
    cron_expr: &str,
    task_type: &str,
    payload: Option<&str>,
) -> Result<i64> {
    let result = sqlx::query(
        "INSERT INTO schedules (user_id, cron_expr, task_type, payload) VALUES (?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind(cron_expr)
    .bind(task_type)
    .bind(payload)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    /// Helper: creates an in-memory DB with users + schedules tables.
    async fn make_pool() -> Pool {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "
            CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                username TEXT,
                role TEXT,
                authorized INTEGER,
                full_name TEXT,
                last_seen_version TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE schedules (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                cron_expr TEXT NOT NULL,
                task_type TEXT NOT NULL,
                payload TEXT,
                last_run TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id)
            );
            CREATE TABLE book_episodes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                approximate_date TEXT,
                content TEXT NOT NULL,
                tags TEXT,
                phase TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id)
            );
            CREATE TABLE book_chapters (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                order_num INTEGER NOT NULL,
                title TEXT NOT NULL,
                filepath TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id)
            );
        ",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO users (id, role, authorized) VALUES (1, 'owner', 1);")
            .execute(&pool)
            .await
            .unwrap();
        pool
    }

    #[tokio::test]
    async fn test_schedule_logic() {
        let pool = make_pool().await;

        // 1. Recurring
        insert_schedule(&pool, 1, "08:00 MON-FRI", "reminder", Some("daily"))
            .await
            .unwrap();
        // 2. One-time for today — use Local::now() to match get_due_schedules behaviour
        let today_date = Local::now().format("%Y-%m-%d").to_string();
        let one_time_expr = format!("{} 10:00", today_date);
        insert_schedule(&pool, 1, &one_time_expr, "reminder", Some("one-time"))
            .await
            .unwrap();
        // 3. One-time for the distant future
        insert_schedule(&pool, 1, "2099-01-01 10:00", "reminder", Some("future"))
            .await
            .unwrap();

        // recurring fires on a weekday
        let due = get_due_schedules(&pool, "08:00", "WED").await.unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].payload.as_deref(), Some("daily"));

        // recurring does NOT fire on weekend
        let not_due = get_due_schedules(&pool, "08:00", "SAT").await.unwrap();
        assert_eq!(not_due.len(), 0);

        // one-time fires today at 10:00
        let due_onetime = get_due_schedules(&pool, "10:00", "WED").await.unwrap();
        assert_eq!(due_onetime.len(), 1);
        assert_eq!(due_onetime[0].payload.as_deref(), Some("one-time"));

        // one-time does NOT fire at wrong time
        let not_due_onetime = get_due_schedules(&pool, "11:00", "WED").await.unwrap();
        assert_eq!(not_due_onetime.len(), 0);
    }

    /// Regression: "08:00 MON-FRI" contains '-' but must NOT be treated as a one-time event.
    /// Before the fix, `cron_expr.contains('-')` wrongly returned true, causing the recurring
    /// reminder to be deleted after its first execution.
    #[tokio::test]
    async fn test_recurring_monfri_not_treated_as_onetime() {
        let pool = make_pool().await;

        insert_schedule(&pool, 1, "09:00 MON-FRI", "reminder", Some("standup"))
            .await
            .unwrap();

        let due = get_due_schedules(&pool, "09:00", "MON").await.unwrap();
        assert_eq!(due.len(), 1, "MON-FRI schedule should fire on Monday");

        let expr = &due[0].cron_expr;
        // Verify it does NOT look like a one-time expression (must not start with YYYY-MM-DD)
        let first_part = expr.split_whitespace().next().unwrap_or("");
        assert_ne!(
            first_part.len(),
            10,
            "A recurring HH:MM expr must not be detected as a date"
        );
        assert!(
            !first_part
                .chars()
                .next()
                .map(|c| c.is_ascii_digit() && first_part.len() == 10)
                .unwrap_or(false),
            "Recurring expression wrongly identified as one-time: {}",
            expr
        );
    }

    /// Regression: date matching must use Local time (not UTC) so reminders in non-UTC
    /// timezones (e.g. UTC-3) are not silently skipped after 21:00 local time.
    #[tokio::test]
    async fn test_timezone_local_date_matching() {
        let pool = make_pool().await;

        // Schedule a one-time reminder for today according to LOCAL date
        let local_today = Local::now().format("%Y-%m-%d").to_string();
        let expr = format!("{} 23:59", local_today);
        insert_schedule(&pool, 1, &expr, "reminder", Some("night-owl"))
            .await
            .unwrap();

        // Must resolve as due on LOCAL today ("23:59") regardless of UTC offset
        let due = get_due_schedules(&pool, "23:59", "MON").await.unwrap();
        assert_eq!(
            due.len(),
            1,
            "Reminder scheduled for local today's date should be found when matching with local date"
        );
        assert_eq!(due[0].payload.as_deref(), Some("night-owl"));
    }
    
    #[tokio::test]
    async fn test_book_writing_queries() {
        let pool = make_pool().await;
        
        let user_id = 1;
        
        // 1. Insert and search episodes
        insert_book_episode(&pool, user_id, Some("Inverno 2018"), "Nevou muito.", Some("inverno,neve"), Some("chegada")).await.unwrap();
        insert_book_episode(&pool, user_id, Some("Verão 2019"), "Dias longos.", Some("verao,sol"), Some("adaptacao")).await.unwrap();
        
        let eps_all = search_book_episodes(&pool, user_id, None, None).await.unwrap();
        assert_eq!(eps_all.len(), 2);
        
        let eps_neve = search_book_episodes(&pool, user_id, Some("neve"), None).await.unwrap();
        assert_eq!(eps_neve.len(), 1);
        assert_eq!(eps_neve[0].content, "Nevou muito.");
        
        let eps_fase = search_book_episodes(&pool, user_id, None, Some("adaptacao")).await.unwrap();
        assert_eq!(eps_fase.len(), 1);
        assert_eq!(eps_fase[0].content, "Dias longos.");
        
        // 2. Insert and get chapters
        insert_book_chapter(&pool, user_id, 2, "O Sol da Meia-noite", "manuscrito/capitulo_02.md").await.unwrap();
        insert_book_chapter(&pool, user_id, 1, "Chegada no Frio", "manuscrito/capitulo_01.md").await.unwrap();
        
        // Ensure they are ordered by order_num ASC
        let chaps = get_book_chapters(&pool, user_id).await.unwrap();
        assert_eq!(chaps.len(), 2);
        assert_eq!(chaps[0].order_num, 1);
        assert_eq!(chaps[1].order_num, 2);
        
        // Replace existing chapter correctly
        insert_book_chapter(&pool, user_id, 1, "A Longa Chegada", "manuscrito/capitulo_01.md").await.unwrap();
        let chaps_after = get_book_chapters(&pool, user_id).await.unwrap();
        assert_eq!(chaps_after.len(), 2, "Should replace, not duplicate");
        assert_eq!(chaps_after[0].title, "A Longa Chegada");
    }
}

// --- BOOK WRITING QUERIES (O Segredo da Suécia) ---

/// Insert a new book episode memory.
pub async fn insert_book_episode(
    pool: &Pool,
    user_id: i64,
    approximate_date: Option<&str>,
    content: &str,
    tags: Option<&str>,
    phase: Option<&str>,
) -> Result<i64> {
    let result = sqlx::query(
        r#"
        INSERT INTO book_episodes (user_id, approximate_date, content, tags, phase)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(user_id)
    .bind(approximate_date)
    .bind(content)
    .bind(tags)
    .bind(phase)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

/// Search book episodes.
pub async fn search_book_episodes(
    pool: &Pool,
    user_id: i64,
    tags: Option<&str>,
    phase: Option<&str>,
) -> Result<Vec<crate::db::models::BookEpisode>> {
    let mut query_str = String::from("SELECT * FROM book_episodes WHERE user_id = ?");
    
    if tags.is_some() {
        query_str.push_str(" AND tags LIKE ?");
    }
    if phase.is_some() {
        query_str.push_str(" AND phase = ?");
    }
    
    query_str.push_str(" ORDER BY created_at ASC");

    let mut query = sqlx::query_as::<_, crate::db::models::BookEpisode>(&query_str).bind(user_id);
    
    if let Some(t) = tags {
        query = query.bind(format!("%{}%", t));
    }
    if let Some(p) = phase {
        query = query.bind(p);
    }
    
    let episodes = query.fetch_all(pool).await?;
    Ok(episodes)
}

/// Insert a new book chapter metadata.
pub async fn insert_book_chapter(
    pool: &Pool,
    user_id: i64,
    order_num: i64,
    title: &str,
    filepath: &str,
) -> Result<i64> {
    // Upsert or replace depending on order? For now let's just insert, but if order_num exists for the same user, we probably should replace it or delete old ones.
    // To make it simple and safe for rewrite: delete existing same order_num for this user first
    sqlx::query("DELETE FROM book_chapters WHERE user_id = ? AND order_num = ?")
        .bind(user_id)
        .bind(order_num)
        .execute(pool)
        .await?;

    let result = sqlx::query(
        r#"
        INSERT INTO book_chapters (user_id, order_num, title, filepath)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(user_id)
    .bind(order_num)
    .bind(title)
    .bind(filepath)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

/// Get all book chapters for a user ordered by order_num.
pub async fn get_book_chapters(pool: &Pool, user_id: i64) -> Result<Vec<crate::db::models::BookChapter>> {
    let chapters = sqlx::query_as::<_, crate::db::models::BookChapter>(
        "SELECT * FROM book_chapters WHERE user_id = ? ORDER BY order_num ASC"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(chapters)
}


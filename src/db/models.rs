//! Database models and entities.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// An interaction record in the database.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Interaction {
    pub id: i64,
    pub chat_id: i64,
    pub role: String,
    pub content: String,
    pub lang: String,
    pub created_at: DateTime<Utc>,
}

/// New interaction data for insertion (without ID and timestamp).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewInteraction {
    pub chat_id: i64,
    pub role: String,
    pub content: String,
    pub lang: String,
}

impl NewInteraction {
    /// Create a new user interaction.
    pub fn user(chat_id: i64, content: String, lang: &str) -> Self {
        Self {
            chat_id,
            role: "user".to_string(),
            content,
            lang: lang.to_string(),
        }
    }

    /// Create a new assistant interaction.
    pub fn assistant(chat_id: i64, content: String, lang: &str) -> Self {
        Self {
            chat_id,
            role: "assistant".to_string(),
            content,
            lang: lang.to_string(),
        }
    }

    /// Create a new system interaction.
    pub fn system(chat_id: i64, content: String, lang: &str) -> Self {
        Self {
            chat_id,
            role: "system".to_string(),
            content,
            lang: lang.to_string(),
        }
    }
}

/// A document record in the database.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Document {
    pub id: i64,
    pub user_id: i64,
    pub filename: String,
    pub path: String,
    pub created_at: DateTime<Utc>,
}

/// A chunk of a document with its embedding.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DocumentChunk {
    pub id: i64,
    pub document_id: i64,
    pub content: String,
    pub embedding: Vec<u8>,
    pub created_at: DateTime<Utc>,
}

/// A vehicle record in the database.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Vehicle {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub model: Option<String>,
    pub plate: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// A fuel log record in the database.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FuelLog {
    pub id: i64,
    pub vehicle_id: i64,
    pub date: DateTime<Utc>,
    pub odometer: f64,
    pub liters: f64,
    pub price_per_liter: f64,
    pub fuel_type: String,
    pub total_cost: f64,
}

/// A general expense record in the database.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ExpenseLog {
    pub id: i64,
    pub vehicle_id: i64,
    pub date: DateTime<Utc>,
    pub category: String,
    pub description: Option<String>,
    pub cost: f64,
}

/// A user record for access control.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i64,
    pub username: Option<String>,
    pub role: String,
    pub authorized: bool,
    pub last_seen_version: String,
    pub full_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn is_authorized(&self) -> bool {
        self.authorized
    }

    pub fn is_admin(&self) -> bool {
        self.role == "owner" || self.role == "admin"
    }
}

/// A scheduled task record.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Schedule {
    pub id: i64,
    pub user_id: i64,
    pub cron_expr: String,
    pub task_type: String,
    pub payload: Option<String>,
    pub last_run: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// A book episode (memory recorded for writing a book).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BookEpisode {
    pub id: i64,
    pub user_id: i64,
    pub approximate_date: Option<String>,
    pub content: String,
    pub tags: Option<String>,
    pub phase: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// A book chapter metadata record.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BookChapter {
    pub id: i64,
    pub user_id: i64,
    pub order_num: i64,
    pub title: String,
    pub filepath: String,
    pub created_at: DateTime<Utc>,
}

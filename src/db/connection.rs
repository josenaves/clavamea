//! Database connection pooling and configuration.

use anyhow::Result;
use sqlx::{Pool as SqlxPool, Sqlite, SqlitePool};

/// Type alias for the SQLite connection pool.
pub type Pool = SqlxPool<Sqlite>;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;

/// Create a connection pool to the SQLite database.
pub async fn create_pool(database_url: &str) -> Result<Pool> {
    let options = SqliteConnectOptions::from_str(database_url)?.create_if_missing(true);
    let pool = SqlitePoolOptions::new().connect_with(options).await?;
    Ok(pool)
}

/// Run database migrations.
pub async fn run_migrations(pool: &Pool) -> Result<()> {
    sqlx::migrate!().run(pool).await?;
    Ok(())
}

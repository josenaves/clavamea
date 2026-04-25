//! Database migration management.

use anyhow::Result;
use sqlx::{Pool as SqlxPool, Sqlite};

/// Run all pending migrations.
pub async fn migrate(_pool: &SqlxPool<Sqlite>) -> Result<()> {
    // TODO: Fix migration path for sqlx::migrate! macro
    // sqlx::migrate!("migrations").run(pool).await?;
    Ok(())
}

/// Rollback the last migration (not implemented yet).
pub async fn rollback(_pool: &SqlxPool<Sqlite>) -> Result<()> {
    // TODO: Implement rollback logic if needed
    Ok(())
}

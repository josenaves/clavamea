//! Database schemas, migrations, and CRUD operations.
//!
//! This module manages all SQLite persistence:
//! - Database connection pooling
//! - Schema migrations
//! - CRUD operations for interactions
//! - Query utilities for conversation history

#![allow(unused_imports)]

pub mod connection;
pub mod migrations;
pub mod models;
pub mod queries;

pub use connection::*;
pub use migrations::*;
pub use models::*;
pub use queries::*;

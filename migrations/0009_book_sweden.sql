-- Migration: 0009_book_sweden.sql
-- Description: Create tables for "O Segredo da Suécia" book project.
-- Idempotent: Using IF NOT EXISTS to prevent errors if migration is partially run.

CREATE TABLE IF NOT EXISTS book_episodes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    approximate_date TEXT,
    content TEXT NOT NULL,
    tags TEXT,
    phase TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS book_chapters (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    order_num INTEGER NOT NULL,
    title TEXT NOT NULL,
    filepath TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

-- Migration: 0005_access_control.sql
-- Description: Create users table for access control and authorization.

CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY, -- Telegram User ID
    username TEXT,
    role TEXT NOT NULL DEFAULT 'pending', -- owner, admin, family, friend, subscriber, pending
    authorized BOOLEAN NOT NULL DEFAULT FALSE,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Note: The owner will be inserted during the first run or manually via admin commands.
-- However, for the initial migration, we'll try to get the owner from the environment if possible
-- but SQL doesn't know about .env. We'll handle seeding in the Application logic or via a manual step.

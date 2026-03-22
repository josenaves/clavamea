-- Migration: 0006_user_version_tracking.sql
-- Description: Track which bot version the user last interacted with,
--              enabling automatic "What's New" changelog notifications.

ALTER TABLE users ADD COLUMN last_seen_version TEXT NOT NULL DEFAULT '';

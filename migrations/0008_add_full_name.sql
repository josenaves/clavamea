-- Migration: 0008_add_full_name.sql
-- Description: Add full_name column to users table for better user identification.

ALTER TABLE users ADD COLUMN full_name TEXT;
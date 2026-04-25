-- Migration: Add search_query column to schedules table
ALTER TABLE schedules ADD COLUMN search_query TEXT;
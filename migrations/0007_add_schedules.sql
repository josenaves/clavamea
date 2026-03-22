-- Migration: Add schedules table for recurring tasks
CREATE TABLE IF NOT EXISTS schedules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    cron_expr TEXT NOT NULL, -- Simplified cron expression (e.g. "17:10 MON-FRI")
    task_type TEXT NOT NULL, -- e.g. "bovespa_clipping"
    payload TEXT,            -- Optional JSON payload
    last_run TEXT,           -- ISO8601 timestamp
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

-- Insert the requested Bovespa clipping task for the owner
-- Note: Replace 171600982 with the actual OWNER_ID from the environment if needed, 
-- but since we already identified it as 171600982, we'll use it.
INSERT OR IGNORE INTO users (id, username, role, authorized) VALUES (171600982, 'Owner', 'owner', 1);

INSERT INTO schedules (user_id, cron_expr, task_type) 
VALUES (171600982, '17:10 MON-FRI', 'bovespa_clipping');

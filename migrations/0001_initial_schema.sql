-- Create interactions table
CREATE TABLE interactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chat_id INTEGER NOT NULL,
    role TEXT CHECK(role IN ('user', 'assistant', 'system')) NOT NULL,
    content TEXT NOT NULL,
    lang TEXT DEFAULT 'en',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Create index for faster chat history retrieval
CREATE INDEX idx_interactions_chat_id ON interactions (chat_id);
CREATE INDEX idx_interactions_created_at ON interactions (created_at);
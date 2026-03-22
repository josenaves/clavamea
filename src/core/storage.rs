//! File-based memory storage system.
//!
//! Handles reading and writing to the OpenClaw-inspired memory files:
//! - MEMORY.md: Long-term curated memory
//! - USER.md: User preferences and working style
//! - SOUL.md: Assistant personality and rules
//! - YYYY-MM-DD.md: Daily notes

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;

pub struct MemoryStorage {
    base_dir: PathBuf,
}

impl MemoryStorage {
    /// Initialize the memory storage system.
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Result<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();
        
        // Ensure the base directory exists
        if !base_dir.exists() {
            fs::create_dir_all(&base_dir)
                .with_context(|| format!("Failed to create memory directory at {:?}", base_dir))?;
        }

        Ok(Self { base_dir })
    }

    /// Get the base directory for a specific user.
    fn user_dir(&self, user_id: i64) -> PathBuf {
        self.base_dir.join(user_id.to_string())
    }

    /// Ensure the core memory files exist for a user.
    pub fn ensure_user_files(&self, user_id: i64) -> Result<()> {
        let user_path = self.user_dir(user_id);
        if !user_path.exists() {
            fs::create_dir_all(&user_path)?;
        }

        let files = vec![
            ("MEMORY.md", "# Long-term Memory\n\nThis file contains curated facts that remain true over time.\n"),
            ("USER.md", "# User Profile\n\nPreferences and working style of the user.\n"),
            ("SOUL.md", "# Assistant Personality\n\nRules and behavior guidelines for ClavaMea.\n"),
        ];

        for (filename, default_content) in files {
            let path = user_path.join(filename);
            if !path.exists() {
                fs::write(&path, default_content)
                    .with_context(|| format!("Failed to create default {:?}", path))?;
            }
        }

        Ok(())
    }

    /// Get the path to today's daily note for a user.
    pub fn daily_note_path(&self, user_id: i64) -> PathBuf {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        self.user_dir(user_id).join(format!("{}.md", today))
    }

    /// Read the contents of a specific memory file for a user.
    pub fn read_file(&self, user_id: i64, filename: &str) -> Result<String> {
        let path = self.user_dir(user_id).join(filename);
        if path.exists() {
            fs::read_to_string(&path)
                .with_context(|| format!("Failed to read memory file {:?}", path))
        } else {
            Ok(String::new())
        }
    }

    /// Append a note to today's daily log for a user.
    pub fn append_daily_note(&self, user_id: i64, content: &str) -> Result<()> {
        use std::io::Write;
        
        self.ensure_user_files(user_id)?;
        let path = self.daily_note_path(user_id);
        
        // If it doesn't exist, start it with a header
        if !path.exists() {
            let today = Utc::now().format("%Y-%m-%d").to_string();
            fs::write(&path, format!("# Daily Notes: {}\n\n", today))?;
        }

        let mut file = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .with_context(|| format!("Failed to open daily note {:?}", path))?;

        let timestamp = Utc::now().format("%H:%M:%S").to_string();
        writeln!(file, "- **{}**: {}", timestamp, content)?;

        Ok(())
    }

    /// Update a memory file by appending content for a user.
    pub fn update_file(&self, user_id: i64, filename: &str, content: &str, append: bool) -> Result<()> {
        self.ensure_user_files(user_id)?;
        let path = self.user_dir(user_id).join(filename);
        
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true);
        
        if append {
            options.append(true);
        } else {
            options.truncate(true);
        }

        let mut file = options.open(&path)
            .with_context(|| format!("Failed to open memory file {:?}", path))?;

        use std::io::Write;
        if append {
            writeln!(file, "{}", content)?;
        } else {
            file.write_all(content.as_bytes())?;
        }

        Ok(())
    }

    /// Build a comprehensive context string combining all core memory files for a user.
    pub fn build_context_string(&self, user_id: i64) -> String {
        let mut context = String::new();

        if let Ok(soul) = self.read_file(user_id, "SOUL.md") {
            if !soul.trim().is_empty() {
                context.push_str(&format!("--- SOUL ---\n{}\n\n", soul.trim()));
            }
        }

        if let Ok(user) = self.read_file(user_id, "USER.md") {
            if !user.trim().is_empty() {
                context.push_str(&format!("--- USER PREFERENCES ---\n{}\n\n", user.trim()));
            }
        }

        // Try reading MEMORY.md first
        let mut long_term_memory = String::new();
        if let Ok(memory) = self.read_file(user_id, "MEMORY.md") {
            if !memory.trim().is_empty() {
                long_term_memory = memory;
            }
        }
        
        if !long_term_memory.is_empty() {
            context.push_str(&format!("--- LONG TERM MEMORY ---\n{}\n\n", long_term_memory.trim()));
        }

        // Include yesterday's notes if they exist
        let yesterday = Utc::now() - chrono::Duration::days(1);
        let yesterday_str = yesterday.format("%Y-%m-%d").to_string();
        let yesterday_path = format!("{}.md", yesterday_str);
        if let Ok(daily_yesterday) = self.read_file(user_id, &yesterday_path) {
            if !daily_yesterday.trim().is_empty() {
                context.push_str(&format!("--- YESTERDAY'S RECENT NOTES ---\n{}\n\n", daily_yesterday.trim()));
            }
        }

        // Include today's daily notes if they exist
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let daily_path = format!("{}.md", today);
        if let Ok(daily) = self.read_file(user_id, &daily_path) {
            if !daily.trim().is_empty() {
                context.push_str(&format!("--- TODAY'S RECENT NOTES ---\n{}\n\n", daily.trim()));
            }
        }

        context
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_memory_storage_initialization() {
        let dir = tempdir().unwrap();
        let storage = MemoryStorage::new(dir.path()).unwrap();
        let user_id = 123;
        storage.ensure_user_files(user_id).unwrap();

        assert!(dir.path().join("123").join("MEMORY.md").exists());
        assert!(dir.path().join("123").join("USER.md").exists());
        assert!(dir.path().join("123").join("SOUL.md").exists());
    }

    #[test]
    fn test_append_daily_note() {
        let dir = tempdir().unwrap();
        let storage = MemoryStorage::new(dir.path()).unwrap();
        let user_id = 123;

        storage.append_daily_note(user_id, "User said hello.").unwrap();
        let note_path = storage.daily_note_path(user_id);
        assert!(note_path.exists());

        let content = fs::read_to_string(note_path).unwrap();
        assert!(content.contains("# Daily Notes:"));
        assert!(content.contains("- **"));
        assert!(content.contains("**: User said hello."));
    }

    #[test]
    fn test_build_context_string() {
        let dir = tempdir().unwrap();
        let storage = MemoryStorage::new(dir.path()).unwrap();
        let user_id = 123;

        storage.ensure_user_files(user_id).unwrap();
        let user_dir = dir.path().join("123");

        // Overwrite default content for testing
        fs::write(user_dir.join("SOUL.md"), "I am a helpful assistant.").unwrap();
        fs::write(user_dir.join("USER.md"), "User likes Rust.").unwrap();
        fs::write(user_dir.join("MEMORY.md"), "The project is ClavaMea.").unwrap();
        storage.append_daily_note(user_id, "Testing context.").unwrap();

        let context = storage.build_context_string(user_id);
        
        // Assert all sections are injected
        assert!(context.contains("--- SOUL ---"));
        assert!(context.contains("I am a helpful assistant."));
        
        assert!(context.contains("--- USER PREFERENCES ---"));
        assert!(context.contains("User likes Rust."));
        
        assert!(context.contains("--- LONG TERM MEMORY ---"));
        assert!(context.contains("The project is ClavaMea."));
        
        assert!(context.contains("--- TODAY'S RECENT NOTES ---"));
        assert!(context.contains("Testing context."));
    }
}

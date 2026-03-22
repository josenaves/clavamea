//! Local Retrieval-Augmented Generation (RAG) system.

use anyhow::Result;
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use std::sync::Arc;
use crate::db::Pool;
use crate::db::models::DocumentChunk;

/// Manages the local RAG system: embedding documents and searching across them.
pub struct RagManager {
    embedding_model: Arc<TextEmbedding>,
    db_pool: Pool,
}

impl RagManager {
    /// Create a new RagManager.
    pub fn new(db_pool: Pool) -> Result<Self> {
        // Initialize the embedding model (this will download it on the first run)
        let mut options = InitOptions::default();
        options.model_name = EmbeddingModel::AllMiniLML6V2;
        options.show_download_progress = true;

        let model = TextEmbedding::try_new(options)?;

        Ok(Self {
            embedding_model: Arc::new(model),
            db_pool,
        })
    }

    /// Ingest a text document into the vector database for a specific user.
    pub async fn ingest_document(&self, user_id: i64, filename: &str, path: &str, content: &str) -> Result<()> {
        // 1. Chunk the content
        let chunks = self.chunk_text(content, 500, 50);

        // 2. Generate embeddings for all chunks
        let embeddings = self.embedding_model.embed(chunks.clone(), None)?;

        // 3. Store document and chunks in the database
        let mut tx = self.db_pool.begin().await?;

        // Insert doc with user_id using runtime query to avoid compile-time schema check
        let doc_id: i64 = sqlx::query_scalar(
            "INSERT INTO documents (user_id, filename, path) VALUES (?, ?, ?) RETURNING id"
        )
        .bind(user_id)
        .bind(filename)
        .bind(path)
        .fetch_one(&mut *tx)
        .await?;

        for (i, chunk_content) in chunks.iter().enumerate() {
            let embedding_bytes = self.vector_to_bytes(&embeddings[i]);
            sqlx::query!(
                "INSERT INTO document_chunks (document_id, content, embedding) VALUES (?, ?, ?)",
                doc_id, chunk_content, embedding_bytes
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Search for relevant document chunks based on a query for a specific user.
    pub async fn search(&self, user_id: i64, query: &str, limit: usize) -> Result<Vec<String>> {
        // 1. Embed the query
        let query_embedding = &self.embedding_model.embed(vec![query], None)?[0];
        
        // 2. Fetch all chunks for this user from the database
        let all_chunks: Vec<DocumentChunk> = sqlx::query_as::<_, DocumentChunk>(
            r#"
            SELECT c.id, c.document_id, c.content, c.embedding, c.created_at 
            FROM document_chunks c 
            JOIN documents d ON c.document_id = d.id 
            WHERE d.user_id = ?
            "#
        )
        .bind(user_id)
        .fetch_all(&self.db_pool)
        .await?;

        // 3. Rank by cosine similarity
        let mut scored_chunks: Vec<(f32, String)> = all_chunks
            .into_iter()
            .map(|chunk| {
                let chunk_vec = self.bytes_to_vector(&chunk.embedding);
                let score = self.cosine_similarity(query_embedding, &chunk_vec);
                (score, chunk.content)
            })
            .collect();

        scored_chunks.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // 4. Return top results
        Ok(scored_chunks
            .into_iter()
            .take(limit)
            .map(|(_, content)| content)
            .collect())
    }

    /// Simple character-based chunking.
    fn chunk_text(&self, text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
        let mut chunks = Vec::new();
        let mut start = 0;
        let text_chars: Vec<char> = text.chars().collect();

        while start < text_chars.len() {
            let end = std::cmp::min(start + chunk_size, text_chars.len());
            let chunk: String = text_chars[start..end].iter().collect();
            chunks.push(chunk);
            
            if end == text_chars.len() {
                break;
            }
            start += chunk_size.saturating_sub(overlap).max(1);
        }

        chunks
    }

    fn vector_to_bytes(&self, vec: &[f32]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(vec.len() * 4);
        for &val in vec {
            bytes.extend_from_slice(&val.to_le_bytes());
        }
        bytes
    }

    fn bytes_to_vector(&self, bytes: &[u8]) -> Vec<f32> {
        bytes
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap_or([0; 4])))
            .collect()
    }

    fn cosine_similarity(&self, v1: &[f32], v2: &[f32]) -> f32 {
        if v1.len() != v2.len() || v1.is_empty() {
            return 0.0;
        }
        let dot_product: f32 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
        let norm_v1: f32 = v1.iter().map(|a| a * a).sum::<f32>().sqrt();
        let norm_v2: f32 = v2.iter().map(|a| a * a).sum::<f32>().sqrt();
        
        if norm_v1 == 0.0 || norm_v2 == 0.0 {
            return 0.0;
        }
        
        dot_product / (norm_v1 * norm_v2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_rag_ingestion_and_search() -> Result<()> {
        // Create a temporary directory for the database
        let dir = tempdir()?;
        let db_path = dir.path().join("test_rag.db");
        let db_url = format!("sqlite:{}", db_path.to_str().unwrap());

        // Initialize database and run migrations
        let pool = connection::create_pool(&db_url).await?;
        connection::run_migrations(&pool).await?;

        // Initialize RAG manager
        let rag = RagManager::new(pool)?;

        // Ingest a test document (user_id = 1 for test)
        let content = "ClavaMea is a private AI assistant. The secret passphrase is: PIZZA_WITH_PINEAPPLE.";
        rag.ingest_document(1, "test.md", "test.md", content).await?;

        // Search for the passphrase
        let results = rag.search(1, "secret passphrase", 1).await?;
        assert!(!results.is_empty(), "Should return at least one result");
        assert!(results[0].contains("PIZZA_WITH_PINEAPPLE"), "Result should contain the passphrase");

        // Search for something else
        let results = rag.search(1, "Who is ClavaMea?", 1).await?;
        assert!(!results.is_empty());
        assert!(results[0].contains("private AI assistant"));

        Ok(())
    }
}


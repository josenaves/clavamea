use sqlx::sqlite::SqlitePool;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pool = SqlitePool::connect("sqlite:data/clavamea.db").await?;
    
    let episodes = sqlx::query!("SELECT id, content, tags, phase FROM book_episodes")
        .fetch_all(&pool)
        .await?;
        
    println!("Found {} episodes:", episodes.len());
    for ep in episodes {
        println!("ID: {}, Phase: {:?}, Tags: {:?}", ep.id, ep.phase, ep.tags);
        println!("Content: {}", ep.content);
        println!("---");
    }
    
    Ok(())
}

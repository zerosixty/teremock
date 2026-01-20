//! Database module with sqlx queries.
//! All functions take PgPool as the first parameter.
pub mod models;

use models::{Phrase, User};
use sqlx::PgPool;

pub type Result<T> = std::result::Result<T, sqlx::Error>;

/// Database migrator for use with #[sqlx::test]
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Create database connection pool
pub async fn create_pool(database_url: &str) -> Result<PgPool> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;
    log::info!("Database connection pool created");
    Ok(pool)
}

/// Run migrations
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await?;
    log::info!("Database migrations completed");
    Ok(())
}

// User queries

pub async fn create_user(pool: &PgPool, id: i64) -> Result<User> {
    sqlx::query_as::<_, User>(
        "INSERT INTO users (id) VALUES ($1) RETURNING *"
    )
    .bind(id)
    .fetch_one(pool)
    .await
}

pub async fn delete_user(pool: &PgPool, id: i64) -> Result<u64> {
    let result = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn get_user(pool: &PgPool, id: i64) -> Result<User> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
}

pub async fn change_user_nickname(pool: &PgPool, id: i64, nickname: String) -> Result<User> {
    sqlx::query_as::<_, User>(
        "UPDATE users SET nickname = $2 WHERE id = $1 RETURNING *"
    )
    .bind(id)
    .bind(nickname)
    .fetch_one(pool)
    .await
}

// Phrase queries

pub async fn get_user_phrases(pool: &PgPool, user_id: i64) -> Result<Vec<Phrase>> {
    sqlx::query_as::<_, Phrase>("SELECT * FROM phrases WHERE user_id = $1")
        .bind(user_id)
        .fetch_all(pool)
        .await
}

pub async fn create_phrase(
    pool: &PgPool,
    user_id: i64,
    emoji: String,
    text: String,
    bot_text: String,
) -> Result<Phrase> {
    sqlx::query_as::<_, Phrase>(
        "INSERT INTO phrases (user_id, emoji, text, bot_text) VALUES ($1, $2, $3, $4) RETURNING *"
    )
    .bind(user_id)
    .bind(emoji)
    .bind(text)
    .bind(bot_text)
    .fetch_one(pool)
    .await
}

pub async fn delete_phrase(pool: &PgPool, id: i32) -> Result<u64> {
    let result = sqlx::query("DELETE FROM phrases WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

// Test helpers

/// For tests: fully reset a user (delete and recreate with optional nickname)
pub async fn full_user_redeletion(pool: &PgPool, id: i64, nickname: Option<String>) -> Result<()> {
    let _ = delete_user(pool, id).await;
    create_user(pool, id).await?;
    if let Some(nick) = nickname {
        change_user_nickname(pool, id, nick).await?;
    }
    Ok(())
}

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use std::env;
use std::str::FromStr;
use std::time::Duration;

pub async fn init_pool() -> Result<SqlitePool, sqlx::Error> {
    let connection_options = SqliteConnectOptions::from_str("sqlite://storage.db")?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal);

    let acquire_timeout_secs = env::var("DB_ACQUIRE_TIMEOUT_SECS")
        .unwrap_or_else(|_| "30".to_string())
        .parse::<u64>()
        .unwrap_or(30);

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .acquire_timeout(Duration::from_secs(acquire_timeout_secs))
        .connect_with(connection_options)
        .await?;

    // Initialize schema
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS requests (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            method TEXT NOT NULL,
            path TEXT NOT NULL,
            content_length INTEGER,
            content_type TEXT,
            user_agent TEXT,
            client_s_ip TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS request_headers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            request_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            value TEXT,
            FOREIGN KEY (request_id) REFERENCES requests (id) ON DELETE CASCADE
        );",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS request_bodies (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            request_id INTEGER NOT NULL,
            body BLOB,
            FOREIGN KEY (request_id) REFERENCES requests (id) ON DELETE CASCADE
        );",
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

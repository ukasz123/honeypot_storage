use super::storage::Storage;
use async_trait::async_trait;
use axum::body::Body;
use axum::http::Request;
use sqlx::{Row, SqlitePool, sqlite::SqliteConnectOptions, sqlite::SqliteJournalMode};
use std::env;
use std::str::FromStr;
use std::time::Duration;
use tracing::error;

#[derive(Clone)]
pub struct SqliteStorage {
    pub pool: SqlitePool,
}

impl SqliteStorage {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
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

        let storage = Self { pool: pool.clone() };
        storage.init().await?;
        Ok(storage)
    }
}

#[async_trait]
impl Storage for SqliteStorage {
    async fn init(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        .execute(&self.pool)
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
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS request_bodies (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                request_id INTEGER NOT NULL,
                body BLOB,
                FOREIGN KEY (request_id) REFERENCES requests (id) ON DELETE CASCADE
            );",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn save_request(
        &self,
        req: Request<Body>,
        client_ip: Option<String>,
        max_body_bytes: usize,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 1. Extract all metadata from the request before consuming it
        let method = req.method().to_string();
        let path = req.uri().path().to_string();

        let content_length = req
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<i64>().ok());

        let content_type = req
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let user_agent = req
            .headers()
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Clone headers for the subsequent insertion loop
        let headers: Vec<(String, String)> = req
            .headers()
            .iter()
            .map(|(n, v)| (n.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        // 2. Collect the body bytes (I/O intensive part) - DO NOT hold a transaction yet
        let body_bytes =
            crate::body_utils::collect_body_with_limit(req.into_body(), max_body_bytes)
                .await
                .map_err(|e| {
                    error!("Failed to read request body: {}", e);
                    format!("Body error: {}", e)
                })?;

        // 3. Now start the transaction only when we have all data ready to be written
        let mut tx = self.pool.begin().await?;

        let id: u32 = sqlx::query(
            "INSERT INTO requests (method, path, content_length, content_type, user_agent, client_s_ip)
             VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(method)
        .bind(path)
        .bind(content_length)
        .bind(content_type)
        .bind(user_agent)
        .bind(client_ip)
        .fetch_one(&mut *tx) // Use the transaction!
        .await?
        .get::<_, _>(0);

        for (name, value) in headers {
            sqlx::query("INSERT INTO request_headers (request_id, name, value) VALUES (?, ?, ?)")
                .bind(id)
                .bind(name)
                .bind(value)
                .execute(&mut *tx) // Use the transaction!
                .await?;
        }

        if !body_bytes.is_empty() {
            sqlx::query("INSERT INTO request_bodies (request_id, body) VALUES (?, ?)")
                .bind(id)
                .bind(body_bytes.to_vec())
                .execute(&mut *tx) // Use the transaction!
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}

use crate::storage::Storage;
use axum::{
    Router,
    body::Body,
    extract::{ConnectInfo, State},
    http::Request,
    routing::any,
};
use std::env;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::{Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};
mod body_utils;
mod db;
#[cfg(feature = "duckdb")]
use db::DuckdbStorage;
#[cfg(feature = "sqlite")]
use db::SqliteStorage;
mod storage;

type CapturedRequest = (Request<Body>, String);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(fmt::layer().with_filter(if cfg!(debug_assertions) {
            LevelFilter::DEBUG
        } else {
            LevelFilter::INFO
        }))
        .init();

    // Read PORT from environment variable, default to 3000
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);

    // Read MAX_BODY_BYTES from environment variable, default to 1MB
    let max_body_bytes = env::var("MAX_BODY_BYTES")
        .unwrap_or_else(|_| "1048576".to_string())
        .parse::<usize>()
        .unwrap_or(1024 * 1024);

    // Read MAX_CONCURRENT_WRITES from environment and default to 2000
    let max_concurrent_writes = env::var("MAX_CONCURRENT_WRITES")
        .unwrap_or_else(|_| "2000".to_string())
        .parse::<usize>()
        .unwrap_or(2000);

    // Initialize Database
    info!("Initializing database...");
    #[cfg(feature = "sqlite")]
    let storage: Arc<dyn Storage> = Arc::new(SqliteStorage::new().await.map_err(|e| {
        error!("Failed to initialize SQLite database: {}", e);
        e as Box<dyn std::error::Error + Send + Sync>
    })?);

    #[cfg(feature = "duckdb")]
    let storage: Arc<dyn Storage> = Arc::new(DuckdbStorage::new().map_err(|e| {
        error!("Failed to initialize DuckDB database: {}", e);
        e as Box<dyn std::error::Error + Send + Sync>
    })?);

    #[cfg(not(any(feature = "sqlite", feature = "duckdb")))]
    panic!("No database feature enabled. Please enable 'sqlite' or 'duckdb' features.");

    storage
        .init()
        .await
        .expect("Storage was unable to initialize");

    // Create channel for background dispatcher
    let (tx, mut rx) = mpsc::channel::<CapturedRequest>(100);

    // Semaphore to limit concurrent database write tasks
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(max_concurrent_writes));
    let worker_pool = storage.clone();

    // Spawn the dispatcher task
    tokio::spawn(async move {
        info!(
            "Dispatcher task started. Max concurrent writes: {}",
            max_concurrent_writes
        );
        while let Some((captured, client_ip)) = rx.recv().await {
            let storage_inner = worker_pool.clone();
            let semaphore_inner = semaphore.clone();
            let max_bytes = max_body_bytes;

            // We acquire the permit HERE in the dispatcher loop.
            // If all permits are taken, this line will block, preventing the
            // spawning of new tasks and stopping the channel from being drained
            // until a slot becomes available.
            let permit = semaphore_inner
                .acquire_owned()
                .await
                .expect("Semaphore closed");

            tokio::spawn(async move {
                // Move the permit into the task so it is released when the task drops.
                let _permit = permit;

                if let Err(e) =
                    save_request(storage_inner.as_ref(), captured, Some(client_ip), max_bytes).await
                {
                    error!("Failed to save request to database: {}", e);
                }
            });
        }
    });

    // Create the router with a capture-all handler
    let app = Router::new()
        .route("/", any(handler))
        .route("/*path", any(handler))
        .with_state(tx)
        .into_make_service_with_connect_info::<std::net::SocketAddr>();

    info!("Listening on {}", addr);

    let listener = TcpListener::bind(&addr).await?;

    if let Err(e) = axum::serve(listener, app).await {
        error!("Server error: {}", e);
    }

    Ok(())
}

async fn handler(
    State(tx): State<mpsc::Sender<CapturedRequest>>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    req: Request<Body>,
) -> axum::http::StatusCode {
    let client_ip = addr.ip().to_string();

    // Use try_send for non-blocking behavior.
    // If the channel is enough to handle spikes without dropping, we proceed.
    // If it's full, we drop the request to avoid backpressure/latency.
    if let Err(e) = tx.try_send((req, client_ip)) {
        error!(
            "Failed to send request to worker (channel full or closed): {}",
            e
        );
    }

    axum::http::StatusCode::NO_CONTENT
}

// Removed unused worker_loop function as it was redundant with the dispatcher task in main.
async fn save_request(
    storage: &dyn Storage,
    req: Request<Body>,
    client_ip: Option<String>,
    max_body_bytes: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    storage.save_request(req, client_ip, max_body_bytes).await?;
    Ok(())
}

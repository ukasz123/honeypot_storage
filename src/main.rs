use axum::{
    Router,
    body::Body,
    extract::{ConnectInfo, State},
    http::Request,
    routing::any,
};
use sqlx::Row;
use std::env;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::{Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};

mod body_utils;
mod db;

type CapturedRequest = (Request<Body>, String);

#[tokio::main]
async fn main() {
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

    // Read MAX_CONCURRENT_WRITES from environment variable, default to 4
    // This limits how many database write tasks are active at once via a Semaphore.
    let max_concurrent_writes = env::var("MAX_CONCURRENT_WRITES")
        .unwrap_or_else(|_| "2000".to_string())
        .parse::<usize>()
        .unwrap_or(2000);

    // Initialize Database (using SqlitePool for better concurrency)
    info!("Initializing database...");
    let pool = match db::init_pool().await {
        Ok(p) => {
            info!("Database connection pool established.");
            p
        }
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            return;
        }
    };

    // Create channel for background dispatcher
    let (tx, mut rx) = mpsc::channel::<CapturedRequest>(100);

    // Semaphore to limit concurrent database write tasks
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(max_concurrent_writes));
    let worker_pool = pool.clone();

    // Spawn the dispatcher task
    tokio::spawn(async move {
        info!(
            "Dispatcher task started. Max concurrent writes: {}",
            max_concurrent_writes
        );
        while let Some((captured, client_ip)) = rx.recv().await {
            let pool_inner = worker_pool.clone();
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
                    save_request(&pool_inner, captured, Some(client_ip), max_bytes).await
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

    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind to {}: {}", addr, e);
            return;
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        error!("Server error: {}", e);
    }
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

async fn worker_loop(
    mut rx: mpsc::Receiver<CapturedRequest>,
    pool: sqlx::SqlitePool,
    max_body_bytes: usize,
) {
    info!("Worker task started.");
    while let Some((captured, client_ip)) = rx.recv().await {
        if let Err(e) = save_request(&pool, captured, Some(client_ip), max_body_bytes).await {
            error!("Failed to save request to database: {}", e);
        } else {
            info!("Request saved to database");
        }
    }
}

async fn save_request(
    pool: &sqlx::SqlitePool,
    req: Request<Body>,
    client_ip: Option<String>,
    max_body_bytes: usize,
) -> Result<(), sqlx::Error> {
    let tx = pool.begin().await?;

    // Extract specific headers
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

    let id: u32 = sqlx::query(
        "INSERT INTO requests (method, path, content_length, content_type, user_agent, client_s_ip)
         VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(req.method().to_string())
    .bind(req.uri().path().to_string())
    .bind(content_length)
    .bind(content_type)
    .bind(user_agent)
    .bind(client_ip)
    .fetch_one(pool)
    .await?
    .get::<_, _>(0);

    for (name, value) in req
        .headers()
        .iter()
        .map(|(n, v)| (n.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect::<Vec<_>>()
    {
        sqlx::query("INSERT INTO request_headers (request_id, name, value) VALUES (?, ?, ?)")
            .bind(id)
            .bind(name)
            .bind(value)
            .execute(pool)
            .await?;
    }

    // Extract and store the body
    let bytes = crate::body_utils::collect_body_with_limit(req.into_body(), max_body_bytes)
        .await
        .map_err(|e| {
            error!("Failed to read request body: {}", e);
            sqlx::Error::Protocol(format!("Body error: {}", e))
        })?;

    if !bytes.is_empty() {
        sqlx::query("INSERT INTO request_bodies (request_id, body) VALUES (?, ?)")
            .bind(id)
            .bind(bytes.to_vec())
            .execute(pool)
            .await?;
    }

    tx.commit().await?;
    Ok(())
}

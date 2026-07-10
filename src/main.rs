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
use tracing::{error, info};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

mod db;

type CapturedRequest = (Request<Body>, String);

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry().with(fmt::layer()).init();

    // Read PORT from environment variable, default to 3000
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);

    // Read MAX_BODY_BYTES from environment variable, default to 1MB
    let max_body_bytes = env::var("MAX_BODY_BYTES")
        .unwrap_or_else(|_| "1048576".to_string())
        .parse::<usize>()
        .unwrap_or(1024 * 1024);

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

    // Create channel for background worker
    let (tx, rx) = mpsc::channel::<CapturedRequest>(100);

    // Spawn the worker task
    let worker_pool = pool.clone();
    tokio::spawn(async move {
        worker_loop(rx, worker_pool, max_body_bytes).await;
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
    // If the channel is full, we drop the request to avoid backpressure/latency.
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
    let bytes = axum::body::to_bytes(req.into_body(), max_body_bytes)
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

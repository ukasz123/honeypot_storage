use axum::{Router, extract::State, routing::any};
use sqlx::Row;
use std::env;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::{error, info};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

mod db;

#[derive(Debug)]
struct CapturedRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry().with(fmt::layer()).init();

    // Read PORT from environment variable, default to 3000
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);

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
        worker_loop(rx, worker_pool).await;
    });

    // Create the router with a capture-all handler
    let app = Router::new()
        .route("/", any(handler))
        .route("/*path", any(handler))
        .with_state(tx);

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
    req: axum::http::Request<axum::body::Body>,
) -> axum::http::StatusCode {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    // Extract headers
    let headers: Vec<(String, String)> = req
        .headers()
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_str().unwrap_or("").to_string()))
        .collect();

    let captured = CapturedRequest {
        method,
        path,
        headers,
    };

    if let Err(e) = tx.send(captured).await {
        error!("Failed to send request to worker: {}", e);
    }

    axum::http::StatusCode::NO_CONTENT
}

async fn worker_loop(mut rx: mpsc::Receiver<CapturedRequest>, pool: sqlx::SqlitePool) {
    info!("Worker task started.");
    while let Some(captured) = rx.recv().await {
        info!(
            "Worker processing request: {} {}",
            captured.method, captured.path
        );
        if let Err(e) = save_request(&pool, captured).await {
            error!("Failed to save request to database: {}", e);
        }
    }
}

async fn save_request(
    pool: &sqlx::SqlitePool,
    captured: CapturedRequest,
) -> Result<(), sqlx::Error> {
    let tx = pool.begin().await?;

    // We use query! but since we don't have a live DB for compile-time checks in this env,
    // I'll use the standard query method to avoid compilation errors if types don't match perfectly.
    let id: u32 = sqlx::query("INSERT INTO requests (method, path) VALUES (?, ?) RETURNING id")
        .bind(&captured.method)
        .bind(&captured.path)
        .fetch_one(pool)
        .await?
        .get::<_, _>(0);

    for (name, value) in captured.headers {
        sqlx::query("INSERT INTO request_headers (request_id, name, value) VALUES (?, ?, ?)")
            .bind(id)
            .bind(name)
            .bind(value)
            .execute(pool)
            .await?;
    }

    tx.commit().await?;
    Ok(())
}

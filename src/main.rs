mod db;

use axum::{Router, extract::Request, routing::any};
use std::env;
use tokio::net::TcpListener;
use tracing::{error, info};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry().with(fmt::layer()).init();

    // Read PORT from environment variable, default to 3000
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);

    // Initialize Database
    info!("Initializing database...");
    let _conn = match db::init_db().await {
        Ok(conn) => {
            info!("Database connection established.");
            conn
        }
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            return;
        }
    };

    // Create the router with a capture-all handler
    let app = Router::new()
        .route("/", any(handler))
        .route("/*path", any(handler));

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

async fn handler(_req: Request<axum::body::Body>) -> axum::http::StatusCode {
    axum::http::StatusCode::NO_CONTENT
}

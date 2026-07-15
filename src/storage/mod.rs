use async_trait::async_trait;
use axum::body::Body;
use axum::http::Request;
use std::sync::Arc;

#[async_trait]
pub trait Storage: Send + Sync {
    async fn init(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn save_request(
        &self,
        req: Request<Body>,
        client_ip: Option<String>,
        max_body_bytes: usize,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

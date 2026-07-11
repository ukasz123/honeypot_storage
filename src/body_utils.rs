use axum::body::{Body, Bytes};
use futures_util::StreamExt;
use std::error::Error;

/// Reads the body of a request up to a maximum number of bytes.
/// Unlike `axum::body::to_bytes`, this function does not return an error if
/// the limit is reached; it simply returns the bytes collected so far.
pub async fn collect_body_with_limit(
    body: Body,
    max_bytes: usize,
) -> Result<Bytes, Box<dyn Error + Send + Sync>> {
    let mut collected = Vec::new();
    let mut body = body.into_data_stream();

    while let Some(frame) = body.next().await {
        let chunk = match frame {
            Ok(c) => c,
            Err(_) => break,
        };
        let chunk_bytes = Bytes::from(chunk);

        if collected.len() + chunk_bytes.len() > max_bytes {
            let remaining = max_bytes - collected.len();
            if remaining > 0 {
                let truncated = chunk_bytes.slice(..remaining);
                collected.extend_from_slice(&truncated);
            }
            break;
        } else {
            collected.extend_from_slice(&chunk_bytes);
        }
    }

    Ok(Bytes::from(collected))
}

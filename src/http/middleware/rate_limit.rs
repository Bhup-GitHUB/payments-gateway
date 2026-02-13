use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use redis::AsyncCommands;

#[derive(Clone)]
pub struct RateLimitState {
    pub redis_client: redis::Client,
    pub max_per_minute: i64,
}

pub async fn enforce(
    State(state): State<RateLimitState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .split(',')
        .next()
        .unwrap_or("unknown")
        .trim()
        .to_string();

    let key = format!(
        "rate:{}:{}",
        ip,
        chrono::Utc::now().format("%Y%m%d%H%M")
    );

    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        let count: i64 = conn.incr(&key, 1).await.unwrap_or(1);
        let _: bool = conn.expire(&key, 120).await.unwrap_or(false);
        if count > state.max_per_minute {
            return Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .body(Body::from("rate limit exceeded"))
                .unwrap_or_else(|_| Response::new(Body::from("rate limit exceeded")));
        }
    }

    next.run(request).await
}

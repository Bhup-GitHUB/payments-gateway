use crate::AppState;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;

pub async fn readiness(State(state): State<AppState>) -> impl IntoResponse {
    let db_ok = sqlx::query("SELECT 1")
        .execute(&state.payment_service.pool)
        .await
        .is_ok();

    let redis_ok = async {
        if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
            let pong: redis::RedisResult<String> = redis::cmd("PING").query_async(&mut conn).await;
            return pong.is_ok();
        }
        false
    }
    .await;

    let worker_hint = state
        .payment_verification_repo
        .due_items(1)
        .await
        .map(|_| true)
        .unwrap_or(false);

    let ok = db_ok && redis_ok;
    let status = if ok {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status,
        Json(serde_json::json!({
            "ready": ok,
            "db": db_ok,
            "redis": redis_ok,
            "verification_repo": worker_hint
        })),
    )
        .into_response()
}

pub async fn liveness() -> impl IntoResponse {
    (axum::http::StatusCode::OK, Json(serde_json::json!({"alive": true}))).into_response()
}

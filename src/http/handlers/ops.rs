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
    let stream_ok = async {
        if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
            let exists: redis::RedisResult<i32> =
                redis::cmd("EXISTS").arg(&state.stream_key).query_async(&mut conn).await;
            return exists.is_ok();
        }
        false
    }
    .await;
    let outbox_ok = sqlx::query("SELECT 1 FROM outbox LIMIT 1")
        .execute(&state.payment_service.pool)
        .await
        .is_ok();

    let ok = db_ok && redis_ok && stream_ok && worker_hint && outbox_ok;
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
            "stream": stream_ok,
            "verification_worker": worker_hint,
            "outbox_repo": outbox_ok
        })),
    )
        .into_response()
}

pub async fn liveness() -> impl IntoResponse {
    (axum::http::StatusCode::OK, Json(serde_json::json!({"alive": true}))).into_response()
}

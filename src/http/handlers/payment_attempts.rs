use crate::AppState;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;
use uuid::Uuid;

pub async fn list_attempts(
    State(state): State<AppState>,
    Path(payment_id): Path<Uuid>,
) -> impl IntoResponse {
    let attempts = match state.payment_attempts_repo.list_by_payment_id(payment_id).await {
        Ok(v) => v,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    let final_status = attempts
        .last()
        .map(|a| a.status.clone())
        .unwrap_or_else(|| "UNKNOWN".to_string());
    let total_latency_ms: i32 = attempts.iter().map(|a| a.latency_ms).sum();

    (
        axum::http::StatusCode::OK,
        Json(serde_json::json!({
            "payment_id": payment_id,
            "total_attempts": attempts.len(),
            "final_status": final_status,
            "total_latency_ms": total_latency_ms,
            "attempts": attempts
        })),
    )
        .into_response()
}

pub async fn get_status_verification(
    State(state): State<AppState>,
    Path(payment_id): Path<Uuid>,
) -> impl IntoResponse {
    match state.payment_verification_repo.get_by_payment_id(payment_id).await {
        Ok(Some(row)) => (axum::http::StatusCode::OK, Json(row)).into_response(),
        Ok(None) => (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "verification record not found"})),
        )
            .into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

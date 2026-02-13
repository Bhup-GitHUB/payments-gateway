use crate::repo::retry_policy_repo::RetryPolicy;
use crate::AppState;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;

pub async fn get_retry_policy(
    State(state): State<AppState>,
    Path(merchant_id): Path<String>,
) -> impl IntoResponse {
    match state.retry_policy_repo.get_for_merchant(&merchant_id).await {
        Ok(policy) => (axum::http::StatusCode::OK, Json(policy)).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn upsert_retry_policy(
    State(state): State<AppState>,
    Path(merchant_id): Path<String>,
    Json(mut policy): Json<RetryPolicy>,
) -> impl IntoResponse {
    policy.merchant_id = merchant_id;
    match state.retry_policy_repo.upsert(policy).await {
        Ok(_) => (axum::http::StatusCode::OK, Json(serde_json::json!({"updated": true}))).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

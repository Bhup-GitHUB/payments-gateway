use crate::AppState;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;

pub async fn enable_segment(
    State(state): State<AppState>,
    Path(segment): Path<String>,
) -> impl IntoResponse {
    match state.bandit_repo.set_enabled(&segment, true).await {
        Ok(_) => (axum::http::StatusCode::OK, Json(serde_json::json!({"segment": segment, "enabled": true}))).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn disable_segment(
    State(state): State<AppState>,
    Path(segment): Path<String>,
) -> impl IntoResponse {
    match state.bandit_repo.set_enabled(&segment, false).await {
        Ok(_) => (axum::http::StatusCode::OK, Json(serde_json::json!({"segment": segment, "enabled": false}))).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn get_state(State(state): State<AppState>) -> impl IntoResponse {
    match state.bandit_repo.list_state().await {
        Ok(rows) => (axum::http::StatusCode::OK, Json(rows)).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

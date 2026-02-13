use crate::AppState;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;

pub async fn status(State(_state): State<AppState>) -> impl IntoResponse {
    (
        axum::http::StatusCode::OK,
        Json(serde_json::json!({"status": "not-configured"})),
    )
        .into_response()
}

pub async fn force_open(
    State(_state): State<AppState>,
    Path((_gateway, _method)): Path<(String, String)>,
) -> impl IntoResponse {
    (
        axum::http::StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({"error": "circuit override not implemented"})),
    )
        .into_response()
}

pub async fn force_close(
    State(_state): State<AppState>,
    Path((_gateway, _method)): Path<(String, String)>,
) -> impl IntoResponse {
    (
        axum::http::StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({"error": "circuit override not implemented"})),
    )
        .into_response()
}

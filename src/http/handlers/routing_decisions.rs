use crate::AppState;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;
use uuid::Uuid;

pub async fn get_routing_decision(
    State(state): State<AppState>,
    Path(payment_id): Path<Uuid>,
) -> impl IntoResponse {
    match state.routing_decisions_repo.get_by_payment_id(payment_id).await {
        Ok(Some(row)) => (axum::http::StatusCode::OK, Json(row)).into_response(),
        Ok(None) => (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "routing decision not found"})),
        )
            .into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

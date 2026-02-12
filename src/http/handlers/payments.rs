use crate::domain::payment::{CreatePaymentRequest, ErrorEnvelope};
use crate::AppState;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::Json;

pub async fn create_payment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreatePaymentRequest>,
) -> impl IntoResponse {
    match state.payment_service.process(req, headers).await {
        Ok(resp) => (axum::http::StatusCode::OK, Json(resp)).into_response(),
        Err((status, body)) => (status, Json(body)).into_response(),
    }
}

pub async fn health() -> impl IntoResponse {
    (axum::http::StatusCode::OK, "ok")
}

pub fn _error_example() -> Json<ErrorEnvelope> {
    Json(ErrorEnvelope {
        error: crate::domain::payment::ErrorPayload {
            code: "EXAMPLE".to_string(),
            message: "example".to_string(),
            details: None,
        },
    })
}

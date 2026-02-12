use crate::AppState;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct GatewayView {
    pub gateway_id: String,
    pub gateway_name: String,
    pub adapter_type: String,
    pub is_enabled: bool,
    pub priority: i32,
    pub supported_methods: Vec<String>,
    pub timeout_ms: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGatewayRequest {
    pub is_enabled: bool,
    pub priority: i32,
    pub supported_methods: Vec<String>,
}

pub async fn list_gateways(State(state): State<AppState>) -> impl IntoResponse {
    match state.gateways_repo.list_all().await {
        Ok(items) => {
            let resp: Vec<GatewayView> = items
                .into_iter()
                .map(|g| GatewayView {
                    gateway_id: g.gateway_id,
                    gateway_name: g.gateway_name,
                    adapter_type: g.adapter_type,
                    is_enabled: g.is_enabled,
                    priority: g.priority,
                    supported_methods: g.supported_methods,
                    timeout_ms: g.timeout_ms,
                })
                .collect();
            (axum::http::StatusCode::OK, Json(resp)).into_response()
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn update_gateway(
    State(state): State<AppState>,
    Path(gateway_id): Path<String>,
    Json(req): Json<UpdateGatewayRequest>,
) -> impl IntoResponse {
    match state
        .gateways_repo
        .update_gateway(
            &gateway_id,
            req.is_enabled,
            req.priority,
            req.supported_methods,
        )
        .await
    {
        Ok(_) => (axum::http::StatusCode::OK, Json(serde_json::json!({"updated": true}))).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

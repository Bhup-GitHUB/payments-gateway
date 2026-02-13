use crate::circuit::state::{CircuitSnapshot, CircuitState};
use crate::circuit::store_redis::CircuitStoreRedis;
use crate::AppState;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CircuitStatusItem {
    pub gateway_id: String,
    pub payment_method: String,
    pub state: CircuitState,
    pub failure_rate_2m: f64,
    pub timeout_rate_5m: f64,
    pub consecutive_failures: i32,
    pub opened_at: Option<chrono::DateTime<chrono::Utc>>,
    pub cooldown_until: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub override_state: Option<String>,
}

pub async fn status(State(state): State<AppState>) -> impl IntoResponse {
    let store = CircuitStoreRedis::new(state.redis_client.clone());
    let gateways = match state.gateways_repo.list_all().await {
        Ok(v) => v,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    let mut items = Vec::new();
    for gateway in gateways {
        for method in gateway.supported_methods {
            let snapshot = store
                .get_snapshot(&gateway.gateway_id, &method)
                .await
                .unwrap_or_else(|_| CircuitSnapshot::new(&gateway.gateway_id, &method));
            let override_state = store.get_override(&gateway.gateway_id, &method).await.ok().flatten();

            items.push(CircuitStatusItem {
                gateway_id: gateway.gateway_id.clone(),
                payment_method: method,
                state: snapshot.state,
                failure_rate_2m: snapshot.failure_rate_2m,
                timeout_rate_5m: snapshot.timeout_rate_5m,
                consecutive_failures: snapshot.consecutive_failures,
                opened_at: snapshot.opened_at,
                cooldown_until: snapshot.cooldown_until,
                updated_at: snapshot.updated_at,
                override_state,
            });
        }
    }

    (axum::http::StatusCode::OK, Json(items)).into_response()
}

pub async fn force_open(
    State(state): State<AppState>,
    Path((gateway, method)): Path<(String, String)>,
) -> impl IntoResponse {
    let store = CircuitStoreRedis::new(state.redis_client.clone());
    let m = method.to_uppercase();
    if let Err(e) = store.set_override(&gateway, &m, "FORCE_OPEN").await {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response();
    }

    let mut snapshot = store
        .get_snapshot(&gateway, &m)
        .await
        .unwrap_or_else(|_| CircuitSnapshot::new(&gateway, &m));
    snapshot.state = CircuitState::Open;
    snapshot.opened_at = Some(chrono::Utc::now());
    snapshot.cooldown_until = Some(chrono::Utc::now() + chrono::Duration::seconds(30));
    snapshot.updated_at = chrono::Utc::now();
    let _ = store.save_snapshot(&snapshot).await;

    (
        axum::http::StatusCode::OK,
        Json(serde_json::json!({"gateway": gateway, "method": m, "override": "FORCE_OPEN"})),
    )
        .into_response()
}

pub async fn force_close(
    State(state): State<AppState>,
    Path((gateway, method)): Path<(String, String)>,
) -> impl IntoResponse {
    let store = CircuitStoreRedis::new(state.redis_client.clone());
    let m = method.to_uppercase();
    if let Err(e) = store.set_override(&gateway, &m, "FORCE_CLOSED").await {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response();
    }

    let mut snapshot = store
        .get_snapshot(&gateway, &m)
        .await
        .unwrap_or_else(|_| CircuitSnapshot::new(&gateway, &m));
    snapshot.state = CircuitState::Closed;
    snapshot.cooldown_until = None;
    snapshot.updated_at = chrono::Utc::now();
    let _ = store.save_snapshot(&snapshot).await;

    (
        axum::http::StatusCode::OK,
        Json(serde_json::json!({"gateway": gateway, "method": m, "override": "FORCE_CLOSED"})),
    )
        .into_response()
}

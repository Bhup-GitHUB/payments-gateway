use crate::metrics::store_redis::MetricsHotStore;
use crate::AppState;
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct MetricsQuery {
    pub window: Option<String>,
    pub payment_method: Option<String>,
    pub issuing_bank: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MetricsRow {
    pub payment_method: String,
    pub issuing_bank: String,
    pub success_rate: f64,
    pub timeout_rate: f64,
    pub avg_latency_ms: i32,
    pub p50_latency_ms: i32,
    pub p95_latency_ms: i32,
    pub p99_latency_ms: i32,
    pub total_requests: u64,
    pub failed_requests: u64,
    pub timeout_requests: u64,
    pub error_counts: std::collections::HashMap<String, u64>,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct MetricsGatewayResponse {
    pub gateway_name: String,
    pub window: String,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub metrics: Vec<MetricsRow>,
}

pub async fn get_gateway_metrics(
    State(state): State<AppState>,
    Path(gateway_name): Path<String>,
    Query(query): Query<MetricsQuery>,
) -> impl IntoResponse {
    let window = parse_window(query.window.as_deref()).unwrap_or(5);
    match read_metrics(
        &state.metrics_hot_store,
        &gateway_name,
        window,
        query.payment_method.as_deref(),
        query.issuing_bank.as_deref(),
    )
    .await
    {
        Ok(metrics) => {
            let resp = MetricsGatewayResponse {
                gateway_name,
                window: format!("{}m", window),
                generated_at: chrono::Utc::now(),
                metrics,
            };
            (axum::http::StatusCode::OK, Json(resp)).into_response()
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

fn parse_window(v: Option<&str>) -> Option<i64> {
    match v {
        Some("1m") => Some(1),
        Some("5m") => Some(5),
        Some("15m") => Some(15),
        Some("60m") => Some(60),
        None => Some(5),
        _ => None,
    }
}

async fn read_metrics(
    hot_store: &MetricsHotStore,
    gateway_name: &str,
    window: i64,
    payment_method: Option<&str>,
    issuing_bank: Option<&str>,
) -> anyhow::Result<Vec<MetricsRow>> {
    let entries = hot_store
        .read_gateway_metrics(gateway_name, window, payment_method, issuing_bank)
        .await?;

    Ok(entries
        .into_iter()
        .map(|(method, bank, metric)| MetricsRow {
            payment_method: method.to_uppercase(),
            issuing_bank: bank.to_uppercase(),
            success_rate: metric.success_rate,
            timeout_rate: metric.timeout_rate,
            avg_latency_ms: metric.avg_latency_ms,
            p50_latency_ms: metric.p50_latency_ms,
            p95_latency_ms: metric.p95_latency_ms,
            p99_latency_ms: metric.p99_latency_ms,
            total_requests: metric.total_requests,
            failed_requests: metric.failed_requests,
            timeout_requests: metric.timeout_requests,
            error_counts: metric.error_counts,
            generated_at: metric.generated_at,
        })
        .collect())
}

use crate::metrics::amount_bucket::from_amount_minor;
use crate::scoring::engine::rank_gateways;
use crate::scoring::metrics_reader::read_metric_for_gateway;
use crate::scoring::types::{GatewayCandidate, ScoreInputs, ScoreWeights};
use crate::AppState;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DebugQuery {
    pub amount_minor: i64,
    pub payment_method: String,
    pub issuing_bank: String,
}

pub async fn scoring_debug(
    State(state): State<AppState>,
    Query(query): Query<DebugQuery>,
) -> impl IntoResponse {
    let method = query.payment_method.to_uppercase();
    let available = match state.gateways_repo.list_enabled_by_method(&method).await {
        Ok(v) => v,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    let weights = match state.payment_service.scoring_config_repo.load_weights().await {
        Ok(w) => ScoreWeights {
            success_rate_weight: w.success_rate_weight,
            latency_weight: w.latency_weight,
            method_affinity_weight: w.method_affinity_weight,
            bank_affinity_weight: w.bank_affinity_weight,
            amount_fit_weight: w.amount_fit_weight,
            time_weight: w.time_weight,
        },
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    let amount_bucket = from_amount_minor(query.amount_minor);
    let mut candidates = Vec::new();
    for gateway in available {
        let metric = match read_metric_for_gateway(
            &state.metrics_hot_store,
            &gateway.gateway_id,
            &method,
            &query.issuing_bank,
        )
        .await
        {
            Ok(m) => m,
            Err(e) => {
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": e.to_string()})),
                )
                    .into_response()
            }
        };

        let method_affinity = state
            .payment_service
            .scoring_config_repo
            .method_affinity(&gateway.gateway_id, &method)
            .await
            .unwrap_or(0.7);
        let amount_fit = state
            .payment_service
            .scoring_config_repo
            .amount_fit(&gateway.gateway_id, &amount_bucket)
            .await
            .unwrap_or(0.7);
        let time_multiplier = state
            .payment_service
            .scoring_config_repo
            .time_multiplier(&gateway.gateway_id, chrono::Utc::now())
            .await
            .unwrap_or(1.0);

        let bank_affinity = if gateway.gateway_name.to_uppercase() == query.issuing_bank.to_uppercase() {
            1.0
        } else {
            0.5
        };

        candidates.push(GatewayCandidate {
            gateway,
            inputs: ScoreInputs {
                success_rate: metric.success_rate,
                p95_latency_ms: metric.p95_latency_ms,
                method_affinity,
                bank_affinity,
                amount_fit,
                time_multiplier,
            },
        });
    }

    let ranked = rank_gateways(&candidates, &weights);
    (
        axum::http::StatusCode::OK,
        Json(serde_json::json!({
            "input": {
                "amount_minor": query.amount_minor,
                "payment_method": method,
                "issuing_bank": query.issuing_bank,
            },
            "ranked": ranked
        })),
    )
        .into_response()
}

use crate::repo::experiments_repo::{CreateExperimentFilterInput, CreateExperimentInput};
use crate::AppState;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;
use uuid::Uuid;

#[derive(Debug, serde::Deserialize)]
pub struct CreateExperimentRequest {
    pub name: String,
    pub traffic_control_pct: i32,
    pub traffic_treatment_pct: i32,
    pub treatment_gateway: String,
    pub start_date: chrono::DateTime<chrono::Utc>,
    pub end_date: Option<chrono::DateTime<chrono::Utc>>,
    pub created_by: String,
    pub filter: CreateExperimentFilterRequest,
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateExperimentFilterRequest {
    pub payment_method: Option<String>,
    pub min_amount_minor: Option<i64>,
    pub max_amount_minor: Option<i64>,
    pub merchant_id: Option<String>,
    pub amount_bucket: Option<String>,
}

pub async fn create_experiment(
    State(state): State<AppState>,
    Json(req): Json<CreateExperimentRequest>,
) -> impl IntoResponse {
    let input = CreateExperimentInput {
        name: req.name,
        traffic_control_pct: req.traffic_control_pct,
        traffic_treatment_pct: req.traffic_treatment_pct,
        treatment_gateway: req.treatment_gateway,
        start_date: req.start_date,
        end_date: req.end_date,
        created_by: req.created_by,
        filter: CreateExperimentFilterInput {
            payment_method: req.filter.payment_method,
            min_amount_minor: req.filter.min_amount_minor,
            max_amount_minor: req.filter.max_amount_minor,
            merchant_id: req.filter.merchant_id,
            amount_bucket: req.filter.amount_bucket,
        },
    };

    match state.experiments_repo.create(input).await {
        Ok(exp) => (axum::http::StatusCode::CREATED, Json(exp)).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn list_experiments(State(state): State<AppState>) -> impl IntoResponse {
    match state.experiments_repo.list().await {
        Ok(rows) => (axum::http::StatusCode::OK, Json(rows)).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn get_results(
    State(state): State<AppState>,
    Path(experiment_id): Path<Uuid>,
) -> impl IntoResponse {
    match state.experiments_repo.results(experiment_id).await {
        Ok(rows) => (axum::http::StatusCode::OK, Json(rows)).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn stop_experiment(
    State(state): State<AppState>,
    Path(experiment_id): Path<Uuid>,
) -> impl IntoResponse {
    match state.experiments_repo.stop(experiment_id).await {
        Ok(_) => (axum::http::StatusCode::OK, Json(serde_json::json!({"stopped": true}))).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

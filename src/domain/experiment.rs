use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experiment {
    pub experiment_id: Uuid,
    pub name: String,
    pub status: String,
    pub traffic_control_pct: i32,
    pub traffic_treatment_pct: i32,
    pub treatment_gateway: String,
    pub start_date: chrono::DateTime<chrono::Utc>,
    pub end_date: Option<chrono::DateTime<chrono::Utc>>,
    pub created_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentFilter {
    pub experiment_id: Uuid,
    pub payment_method: Option<String>,
    pub min_amount_minor: Option<i64>,
    pub max_amount_minor: Option<i64>,
    pub merchant_id: Option<String>,
    pub amount_bucket: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResultRow {
    pub experiment_id: Uuid,
    pub variant: String,
    pub date_hour: chrono::DateTime<chrono::Utc>,
    pub total_requests: i64,
    pub successful_requests: i64,
    pub failed_requests: i64,
    pub avg_latency_ms: i32,
    pub p95_latency_ms: i32,
    pub total_revenue_minor: i64,
}

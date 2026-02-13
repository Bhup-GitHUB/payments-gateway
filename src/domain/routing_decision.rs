use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecisionRecord {
    pub payment_id: Uuid,
    pub selected_gateway: String,
    pub selected_score: f64,
    pub runner_up_gateway: Option<String>,
    pub runner_up_score: Option<f64>,
    pub strategy: String,
    pub reason_summary: String,
    pub score_breakdown_json: serde_json::Value,
    pub ranked_gateways_json: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

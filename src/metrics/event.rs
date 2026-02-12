use crate::domain::payment::PaymentStatus;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentEvent {
    pub payment_id: Uuid,
    pub gateway_used: String,
    pub payment_method: String,
    pub issuing_bank: String,
    pub amount_bucket: String,
    pub status: PaymentStatus,
    pub latency_ms: i32,
    pub error_code: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

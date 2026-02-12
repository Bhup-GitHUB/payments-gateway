use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentMethod {
    Upi,
    Card,
    Netbanking,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardDetails {
    pub number: String,
    pub exp_month: u8,
    pub exp_year: u16,
    pub cvv: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpiDetails {
    pub vpa: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetbankingDetails {
    pub bank_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentInstrument {
    Card(CardDetails),
    Upi(UpiDetails),
    Netbanking(NetbankingDetails),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreatePaymentRequest {
    pub amount_minor: i64,
    pub currency: String,
    pub payment_method: PaymentMethod,
    pub merchant_id: String,
    pub instrument: PaymentInstrument,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentStatus {
    Success,
    Failure,
    Timeout,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreatePaymentResponse {
    pub payment_id: Uuid,
    pub status: PaymentStatus,
    pub gateway_used: String,
    pub transaction_ref: Option<String>,
    pub routing_strategy: String,
    pub routing_reason: String,
    pub latency_ms: i32,
}

#[derive(Debug, Serialize)]
pub struct ErrorEnvelope {
    pub error: ErrorPayload,
}

#[derive(Debug, Serialize)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

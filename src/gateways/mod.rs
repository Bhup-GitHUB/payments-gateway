use crate::domain::context::PaymentContext;
use crate::domain::payment::PaymentStatus;
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub mod mock;
pub mod razorpay;

#[derive(Debug, Clone)]
pub struct GatewayRequest {
    pub amount_minor: i64,
    pub currency: String,
    pub merchant_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedGatewayResponse {
    pub status: PaymentStatus,
    pub transaction_id: Option<String>,
    pub auth_code: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub gateway_response_code: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GatewayResult {
    pub gateway_used: String,
    pub response: NormalizedGatewayResponse,
}

#[derive(Debug, Clone)]
pub struct GatewayConfig {
    pub gateway_id: String,
    pub gateway_name: String,
    pub adapter_type: String,
    pub is_enabled: bool,
    pub priority: i32,
    pub supported_methods: Vec<String>,
    pub timeout_ms: i32,
    pub mock_behavior: Option<String>,
}

#[async_trait::async_trait]
pub trait PaymentGateway: Send + Sync {
    fn name(&self) -> &'static str;

    async fn initiate_payment(
        &self,
        context: &PaymentContext,
        request: GatewayRequest,
    ) -> Result<GatewayResult>;
}

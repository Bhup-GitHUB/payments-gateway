use crate::domain::context::PaymentContext;
use crate::domain::payment::PaymentStatus;
use crate::gateways::{GatewayRequest, GatewayResult, NormalizedGatewayResponse, PaymentGateway};
use anyhow::Result;

pub struct MockGateway {
    pub gateway_name: String,
    pub behavior: String,
}

#[async_trait::async_trait]
impl PaymentGateway for MockGateway {
    fn name(&self) -> &'static str {
        "mock"
    }

    async fn initiate_payment(
        &self,
        _context: &PaymentContext,
        _request: GatewayRequest,
    ) -> Result<GatewayResult> {
        let response = match self.behavior.as_str() {
            "ALWAYS_FAILURE" => NormalizedGatewayResponse {
                status: PaymentStatus::Failure,
                transaction_id: None,
                auth_code: None,
                error_code: Some("MOCK_DECLINED".to_string()),
                error_message: Some("mock decline".to_string()),
                gateway_response_code: Some("400".to_string()),
            },
            "ALWAYS_TIMEOUT" => NormalizedGatewayResponse {
                status: PaymentStatus::Timeout,
                transaction_id: None,
                auth_code: None,
                error_code: Some("MOCK_TIMEOUT".to_string()),
                error_message: Some("mock timeout".to_string()),
                gateway_response_code: Some("504".to_string()),
            },
            _ => NormalizedGatewayResponse {
                status: PaymentStatus::Success,
                transaction_id: Some(format!("mock_txn_{}", uuid::Uuid::new_v4())),
                auth_code: Some("MOCK_AUTH".to_string()),
                error_code: None,
                error_message: None,
                gateway_response_code: Some("200".to_string()),
            },
        };

        Ok(GatewayResult {
            gateway_used: self.gateway_name.clone(),
            response,
        })
    }
}

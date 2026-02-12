use crate::domain::context::PaymentContext;
use crate::domain::payment::PaymentStatus;
use crate::gateways::{GatewayRequest, GatewayResult, NormalizedGatewayResponse, PaymentGateway};
use anyhow::Result;
use reqwest::StatusCode;
use serde_json::json;

pub struct RazorpayGateway {
    pub base_url: String,
    pub key_id: String,
    pub key_secret: String,
    pub timeout_ms: u64,
    pub client: reqwest::Client,
}

#[async_trait::async_trait]
impl PaymentGateway for RazorpayGateway {
    fn name(&self) -> &'static str {
        "razorpay"
    }

    async fn initiate_payment(
        &self,
        _context: &PaymentContext,
        request: GatewayRequest,
    ) -> Result<GatewayResult> {
        let order_url = format!("{}/v1/orders", self.base_url);
        let body = json!({
            "amount": request.amount_minor,
            "currency": request.currency,
            "receipt": format!("m_{}", request.merchant_id),
            "payment_capture": 1
        });

        let resp = self
            .client
            .post(order_url)
            .basic_auth(&self.key_id, Some(&self.key_secret))
            .json(&body)
            .timeout(std::time::Duration::from_millis(self.timeout_ms))
            .send()
            .await;

        let result = match resp {
            Ok(r) if r.status().is_success() => {
                let v: serde_json::Value = r.json().await.unwrap_or_default();
                NormalizedGatewayResponse {
                    status: PaymentStatus::Success,
                    transaction_id: v.get("id").and_then(|id| id.as_str()).map(ToString::to_string),
                    auth_code: None,
                    error_code: None,
                    error_message: None,
                    gateway_response_code: Some("200".to_string()),
                }
            }
            Ok(r) => {
                let status = r.status();
                let body = r.text().await.unwrap_or_default();
                NormalizedGatewayResponse {
                    status: if status == StatusCode::REQUEST_TIMEOUT {
                        PaymentStatus::Timeout
                    } else {
                        PaymentStatus::Failure
                    },
                    transaction_id: None,
                    auth_code: None,
                    error_code: Some(format!("HTTP_{}", status.as_u16())),
                    error_message: Some(body.chars().take(200).collect()),
                    gateway_response_code: Some(status.as_u16().to_string()),
                }
            }
            Err(e) if e.is_timeout() => NormalizedGatewayResponse {
                status: PaymentStatus::Timeout,
                transaction_id: None,
                auth_code: None,
                error_code: Some("TIMEOUT".to_string()),
                error_message: Some("gateway timeout".to_string()),
                gateway_response_code: Some("504".to_string()),
            },
            Err(e) => NormalizedGatewayResponse {
                status: PaymentStatus::Failure,
                transaction_id: None,
                auth_code: None,
                error_code: Some("NETWORK_ERROR".to_string()),
                error_message: Some(e.to_string()),
                gateway_response_code: None,
            },
        };

        Ok(GatewayResult {
            gateway_used: "razorpay_real".to_string(),
            response: result,
        })
    }
}

use crate::domain::context::build_context;
use crate::domain::payment::{CreatePaymentRequest, CreatePaymentResponse, ErrorEnvelope, ErrorPayload, PaymentStatus};
use crate::gateways::mock::MockGateway;
use crate::gateways::razorpay::RazorpayGateway;
use crate::gateways::{GatewayRequest, PaymentGateway};
use crate::metrics::amount_bucket::from_amount_minor;
use crate::metrics::event::PaymentEvent;
use crate::repo::gateways_repo::GatewaysRepo;
use crate::repo::outbox_repo::OutboxRepo;
use crate::repo::payments_repo::{PaymentRecordInput, PaymentsRepo};
use crate::router::round_robin::RoundRobinRouter;
use axum::http::HeaderMap;
use sqlx::PgPool;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

#[derive(Clone)]
pub struct PaymentService {
    pub pool: PgPool,
    pub payments_repo: PaymentsRepo,
    pub outbox_repo: OutboxRepo,
    pub gateways_repo: GatewaysRepo,
    pub router: Arc<RoundRobinRouter>,
    pub razorpay: Arc<RazorpayGateway>,
}

impl PaymentService {
    pub async fn process(
        &self,
        req: CreatePaymentRequest,
        headers: HeaderMap,
    ) -> Result<CreatePaymentResponse, (axum::http::StatusCode, ErrorEnvelope)> {
        validate_request(&req)?;

        let idempotency_key = headers
            .get("Idempotency-Key")
            .and_then(|h| h.to_str().ok())
            .map(str::to_string)
            .ok_or_else(|| {
                (
                    axum::http::StatusCode::BAD_REQUEST,
                    err("MISSING_IDEMPOTENCY_KEY", "Idempotency-Key header is required"),
                )
            })?;

        let request_hash = hash_request(&req);
        if let Some(found) = self
            .payments_repo
            .find_by_idempotency(&req.merchant_id, &idempotency_key)
            .await
            .map_err(internal)?
        {
            if found.request_hash != request_hash {
                return Err((
                    axum::http::StatusCode::CONFLICT,
                    err(
                        "IDEMPOTENCY_KEY_REUSED_WITH_DIFFERENT_PAYLOAD",
                        "payload does not match original request",
                    ),
                ));
            }

            return Ok(CreatePaymentResponse {
                payment_id: found.payment_id,
                status: parse_status(&found.status),
                gateway_used: found.gateway_used,
                transaction_ref: found.gateway_transaction_ref,
                routing_strategy: found.routing_strategy,
                routing_reason: found.routing_reason,
                latency_ms: found.latency_ms,
            });
        }

        let client_ip = headers
            .get("x-forwarded-for")
            .and_then(|h| h.to_str().ok())
            .map(str::to_string);
        let user_agent = headers
            .get(axum::http::header::USER_AGENT)
            .and_then(|h| h.to_str().ok())
            .map(str::to_string);
        let context = build_context(&req, client_ip, user_agent);

        let method = format!("{:?}", req.payment_method).to_uppercase();
        let available = self
            .gateways_repo
            .list_enabled_by_method(&method)
            .await
            .map_err(internal)?;

        if available.is_empty() {
            return Err((
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                err("NO_GATEWAY_AVAILABLE", "no enabled gateway for payment method"),
            ));
        }

        let (selected, routing_reason) = self.router.select(&available).ok_or_else(|| {
            (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                err("ROUTER_SELECTION_FAILED", "failed to select gateway"),
            )
        })?;

        let gateway_request = GatewayRequest {
            amount_minor: req.amount_minor,
            currency: req.currency.clone(),
            merchant_id: req.merchant_id.clone(),
        };

        let start = Instant::now();
        let gateway_result = if selected.adapter_type == "RAZORPAY" {
            self.razorpay
                .initiate_payment(&context, gateway_request)
                .await
                .map_err(internal)?
        } else {
            let mock = MockGateway {
                gateway_name: selected.gateway_id.clone(),
                behavior: selected
                    .mock_behavior
                    .clone()
                    .unwrap_or_else(|| "ALWAYS_SUCCESS".to_string()),
            };
            mock.initiate_payment(&context, gateway_request)
                .await
                .map_err(internal)?
        };

        let latency_ms = start.elapsed().as_millis() as i32;
        let payment_id = Uuid::new_v4();

        let payment_input = PaymentRecordInput {
            payment_id,
            merchant_id: req.merchant_id.clone(),
            idempotency_key,
            request_hash,
            req: req.clone(),
            issuing_bank: context.issuing_bank.clone(),
            gateway_used: gateway_result.gateway_used.clone(),
            routing_strategy: "ROUND_ROBIN".to_string(),
            routing_reason: routing_reason.clone(),
            status: gateway_result.response.status.clone(),
            gateway_transaction_ref: gateway_result.response.transaction_id.clone(),
            gateway_response_code: gateway_result.response.gateway_response_code.clone(),
            error_message: gateway_result.response.error_message.clone(),
            latency_ms,
        };

        let event = PaymentEvent {
            payment_id,
            gateway_used: gateway_result.gateway_used.clone(),
            payment_method: method.clone(),
            issuing_bank: context
                .issuing_bank
                .clone()
                .unwrap_or_else(|| "UNKNOWN".to_string())
                .to_uppercase(),
            amount_bucket: from_amount_minor(req.amount_minor),
            status: gateway_result.response.status.clone(),
            latency_ms,
            error_code: gateway_result.response.error_code.clone(),
            timestamp: chrono::Utc::now(),
        };

        let mut tx = self.pool.begin().await.map_err(|e| internal(e.into()))?;
        PaymentsRepo::insert_payment_tx(&mut tx, &payment_input)
            .await
            .map_err(internal)?;
        OutboxRepo::insert_tx(
            &mut tx,
            payment_id,
            "payment.attempted",
            serde_json::to_value(event).map_err(|e| internal(e.into()))?,
        )
        .await
        .map_err(internal)?;
        tx.commit().await.map_err(|e| internal(e.into()))?;

        Ok(CreatePaymentResponse {
            payment_id,
            status: gateway_result.response.status,
            gateway_used: gateway_result.gateway_used,
            transaction_ref: gateway_result.response.transaction_id,
            routing_strategy: "ROUND_ROBIN".to_string(),
            routing_reason,
            latency_ms,
        })
    }
}

fn validate_request(req: &CreatePaymentRequest) -> Result<(), (axum::http::StatusCode, ErrorEnvelope)> {
    if req.amount_minor <= 0 {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            err("INVALID_AMOUNT", "amount_minor must be > 0"),
        ));
    }
    if req.currency != "INR" {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            err("INVALID_CURRENCY", "only INR supported in phase 1/2"),
        ));
    }
    Ok(())
}

fn hash_request(req: &CreatePaymentRequest) -> String {
    let s = serde_json::to_string(req).unwrap_or_default();
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn parse_status(s: &str) -> PaymentStatus {
    match s {
        "SUCCESS" => PaymentStatus::Success,
        "TIMEOUT" => PaymentStatus::Timeout,
        _ => PaymentStatus::Failure,
    }
}

fn err(code: &str, message: &str) -> ErrorEnvelope {
    ErrorEnvelope {
        error: ErrorPayload {
            code: code.to_string(),
            message: message.to_string(),
            details: None,
        },
    }
}

fn internal(e: anyhow::Error) -> (axum::http::StatusCode, ErrorEnvelope) {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        err("INTERNAL_ERROR", &e.to_string()),
    )
}

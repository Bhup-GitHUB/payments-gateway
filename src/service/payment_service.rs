use crate::circuit::evaluator::pre_call_decision;
use crate::circuit::state::CircuitDecision;
use crate::circuit::store_redis::CircuitStoreRedis;
use crate::circuit::transitions::apply_transition;
use crate::domain::context::build_context;
use crate::domain::payment::PaymentInstrument;
use crate::domain::payment::{CreatePaymentRequest, CreatePaymentResponse, ErrorEnvelope, ErrorPayload, PaymentStatus};
use crate::gateways::mock::MockGateway;
use crate::gateways::razorpay::RazorpayGateway;
use crate::gateways::{GatewayRequest, PaymentGateway};
use crate::metrics::amount_bucket::from_amount_minor;
use crate::metrics::event::PaymentEvent;
use crate::metrics::store_redis::MetricsHotStore;
use crate::repo::circuit_breaker_config_repo::CircuitBreakerConfigRepo;
use crate::repo::gateways_repo::GatewaysRepo;
use crate::repo::outbox_repo::OutboxRepo;
use crate::repo::payments_repo::{PaymentRecordInput, PaymentsRepo};
use crate::repo::routing_decisions_repo::RoutingDecisionsRepo;
use crate::repo::scoring_config_repo::ScoringConfigRepo;
use crate::scoring::engine::rank_gateways;
use crate::scoring::metrics_reader::read_metric_for_gateway;
use crate::scoring::types::{GatewayCandidate, RankedGateway, ScoreInputs, ScoreWeights};
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
    pub scoring_config_repo: ScoringConfigRepo,
    pub routing_decisions_repo: RoutingDecisionsRepo,
    pub circuit_breaker_config_repo: CircuitBreakerConfigRepo,
    pub metrics_hot_store: MetricsHotStore,
    pub circuit_store: CircuitStoreRedis,
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

        let issuing_bank = self.resolve_issuing_bank(&req, &context).await.map_err(internal)?;
        let amount_bucket = from_amount_minor(req.amount_minor);
        let weights = self.scoring_config_repo.load_weights().await.map_err(internal)?;

        let mut candidates = Vec::new();
        for gateway in available {
            let metric = read_metric_for_gateway(
                &self.metrics_hot_store,
                &gateway.gateway_id,
                &method,
                &issuing_bank,
            )
            .await
            .map_err(internal)?;

            let method_affinity = self
                .scoring_config_repo
                .method_affinity(&gateway.gateway_id, &method)
                .await
                .map_err(internal)?;
            let amount_fit = self
                .scoring_config_repo
                .amount_fit(&gateway.gateway_id, &amount_bucket)
                .await
                .map_err(internal)?;
            let time_multiplier = self
                .scoring_config_repo
                .time_multiplier(&gateway.gateway_id, chrono::Utc::now())
                .await
                .map_err(internal)?;

            let bank_affinity = if gateway.gateway_name.to_uppercase() == issuing_bank {
                1.0
            } else if issuing_bank == "UNKNOWN" {
                0.6
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

        let ranked = rank_gateways(&candidates, &to_weights(&weights));
        if ranked.is_empty() {
            return Err((
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                err("ROUTER_SELECTION_FAILED", "failed to score gateway candidates"),
            ));
        }

        let (selected, selected_was_probe) = self
            .pick_circuit_eligible(&ranked, &method)
            .await
            .map_err(internal)?
            .ok_or_else(|| {
                (
                    axum::http::StatusCode::SERVICE_UNAVAILABLE,
                    err("NO_GATEWAY_AVAILABLE", "all candidate gateways are circuit-open"),
                )
            })?;

        let selected_gateway = candidates
            .iter()
            .find(|c| c.gateway.gateway_id == selected.gateway_id)
            .map(|c| c.gateway.clone())
            .ok_or_else(|| {
                (
                    axum::http::StatusCode::SERVICE_UNAVAILABLE,
                    err("ROUTER_SELECTION_FAILED", "missing selected candidate"),
                )
            })?;

        let gateway_request = GatewayRequest {
            amount_minor: req.amount_minor,
            currency: req.currency.clone(),
            merchant_id: req.merchant_id.clone(),
        };

        let start = Instant::now();
        let gateway_result = if selected_gateway.adapter_type == "RAZORPAY" {
            self.razorpay
                .initiate_payment(&context, gateway_request)
                .await
                .map_err(internal)?
        } else {
            let mock = MockGateway {
                gateway_name: selected_gateway.gateway_id.clone(),
                behavior: selected_gateway
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
        let routing_reason = format!(
            "top_score={:.4}, runner_up={}",
            selected.score,
            ranked
                .get(1)
                .map(|r| r.gateway_id.clone())
                .unwrap_or_else(|| "none".to_string())
        );

        let payment_input = PaymentRecordInput {
            payment_id,
            merchant_id: req.merchant_id.clone(),
            idempotency_key,
            request_hash,
            req: req.clone(),
            issuing_bank: Some(issuing_bank.clone()),
            gateway_used: gateway_result.gateway_used.clone(),
            routing_strategy: "SCORING_ENGINE".to_string(),
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
            issuing_bank: issuing_bank.clone(),
            amount_bucket,
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

        self.routing_decisions_repo
            .insert(
                payment_id,
                &selected.gateway_id,
                selected.score,
                ranked.get(1).map(|r| r.gateway_id.as_str()),
                ranked.get(1).map(|r| r.score),
                "SCORING_ENGINE",
                &routing_reason,
                serde_json::to_value(&selected.breakdown).map_err(|e| internal(e.into()))?,
                serde_json::to_value(&ranked).map_err(|e| internal(e.into()))?,
            )
            .await
            .map_err(internal)?;

        self.update_circuit_state(
            &selected.gateway_id,
            &method,
            &format!("{:?}", gateway_result.response.status).to_uppercase(),
            selected_was_probe,
        )
        .await
        .map_err(internal)?;

        Ok(CreatePaymentResponse {
            payment_id,
            status: gateway_result.response.status,
            gateway_used: gateway_result.gateway_used,
            transaction_ref: gateway_result.response.transaction_id,
            routing_strategy: "SCORING_ENGINE".to_string(),
            routing_reason,
            latency_ms,
        })
    }

    async fn pick_circuit_eligible(
        &self,
        ranked: &[RankedGateway],
        method: &str,
    ) -> anyhow::Result<Option<(RankedGateway, bool)>> {
        for candidate in ranked {
            let override_state = self
                .circuit_store
                .get_override(&candidate.gateway_id, method)
                .await?;

            if override_state.as_deref() == Some("FORCE_OPEN") {
                continue;
            }
            if override_state.as_deref() == Some("FORCE_CLOSED") {
                return Ok(Some((candidate.clone(), false)));
            }

            let thresholds = self
                .circuit_breaker_config_repo
                .get_thresholds(&candidate.gateway_id, method)
                .await?;
            let snapshot = self
                .circuit_store
                .get_snapshot(&candidate.gateway_id, method)
                .await?;

            match pre_call_decision(&snapshot, &thresholds, chrono::Utc::now()) {
                CircuitDecision::Allow => return Ok(Some((candidate.clone(), false))),
                CircuitDecision::Probe => return Ok(Some((candidate.clone(), true))),
                CircuitDecision::Reject(_) => {}
            }
        }

        Ok(None)
    }

    async fn update_circuit_state(
        &self,
        gateway_id: &str,
        method: &str,
        status: &str,
        was_probe: bool,
    ) -> anyhow::Result<()> {
        let now = chrono::Utc::now();
        self.circuit_store
            .write_result(gateway_id, method, status, now)
            .await?;

        let snapshot = self.circuit_store.get_snapshot(gateway_id, method).await?;
        let thresholds = self
            .circuit_breaker_config_repo
            .get_thresholds(gateway_id, method)
            .await?;
        let (failure_rate_2m, _) = self
            .circuit_store
            .aggregate_window(gateway_id, method, 2, now)
            .await?;
        let (_, timeout_rate_5m) = self
            .circuit_store
            .aggregate_window(gateway_id, method, 5, now)
            .await?;

        let updated = apply_transition(
            snapshot,
            &thresholds,
            failure_rate_2m,
            timeout_rate_5m,
            status,
            was_probe,
            now,
        );

        self.circuit_store.save_snapshot(&updated).await?;
        Ok(())
    }

    async fn resolve_issuing_bank(
        &self,
        req: &CreatePaymentRequest,
        context: &crate::domain::context::PaymentContext,
    ) -> anyhow::Result<String> {
        if let PaymentInstrument::Card(card) = &req.instrument {
            if let Some(bank) = self.scoring_config_repo.resolve_bank_from_bin(&card.number).await? {
                return Ok(bank.to_uppercase());
            }
        }

        Ok(context
            .issuing_bank
            .clone()
            .unwrap_or_else(|| "UNKNOWN".to_string())
            .to_uppercase())
    }
}

fn to_weights(w: &crate::repo::scoring_config_repo::ScoringWeights) -> ScoreWeights {
    ScoreWeights {
        success_rate_weight: w.success_rate_weight,
        latency_weight: w.latency_weight,
        method_affinity_weight: w.method_affinity_weight,
        bank_affinity_weight: w.bank_affinity_weight,
        amount_fit_weight: w.amount_fit_weight,
        time_weight: w.time_weight,
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

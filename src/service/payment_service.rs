use crate::circuit::evaluator::pre_call_decision;
use crate::circuit::state::CircuitDecision;
use crate::circuit::store_redis::CircuitStoreRedis;
use crate::circuit::transitions::apply_transition;
use crate::domain::context::build_context;
use crate::experiments::assigner::assign_variant;
use crate::experiments::filter::{matches as experiment_matches, MatchInput as ExperimentMatchInput};
use crate::domain::payment::PaymentInstrument;
use crate::domain::payment::{CreatePaymentRequest, CreatePaymentResponse, ErrorEnvelope, ErrorPayload, PaymentStatus};
use crate::gateways::mock::MockGateway;
use crate::gateways::razorpay::RazorpayGateway;
use crate::gateways::{GatewayConfig, GatewayRequest, GatewayResult, NormalizedGatewayResponse, PaymentGateway};
use crate::metrics::amount_bucket::from_amount_minor;
use crate::metrics::event::PaymentEvent;
use crate::metrics::store_redis::MetricsHotStore;
use crate::repo::circuit_breaker_config_repo::CircuitBreakerConfigRepo;
use crate::repo::error_classification_repo::ErrorClassificationRepo;
use crate::repo::bandit_repo::BanditRepo;
use crate::repo::experiments_repo::ExperimentsRepo;
use crate::repo::gateways_repo::GatewaysRepo;
use crate::repo::outbox_repo::OutboxRepo;
use crate::repo::payment_attempts_repo::{NewPaymentAttempt, PaymentAttemptsRepo};
use crate::repo::payment_verification_repo::PaymentVerificationRepo;
use crate::repo::payments_repo::{PaymentRecordInput, PaymentsRepo};
use crate::repo::retry_policy_repo::RetryPolicyRepo;
use crate::repo::routing_decisions_repo::RoutingDecisionsRepo;
use crate::repo::scoring_config_repo::ScoringConfigRepo;
use crate::scoring::engine::rank_gateways;
use crate::scoring::metrics_reader::read_metric_for_gateway;
use crate::scoring::types::{GatewayCandidate, RankedGateway, ScoreInputs, ScoreWeights};
use crate::service::retry_orchestrator::{attempt_limit, classify_attempt_result, should_stop_for_budget, RetryDirective};
use crate::service::webhook_dispatcher::WebhookDispatcher;
use axum::http::HeaderMap;
use chrono::Timelike;
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
    pub experiments_repo: ExperimentsRepo,
    pub bandit_repo: BanditRepo,
    pub scoring_config_repo: ScoringConfigRepo,
    pub routing_decisions_repo: RoutingDecisionsRepo,
    pub circuit_breaker_config_repo: CircuitBreakerConfigRepo,
    pub metrics_hot_store: MetricsHotStore,
    pub circuit_store: CircuitStoreRedis,
    pub payment_attempts_repo: PaymentAttemptsRepo,
    pub retry_policy_repo: RetryPolicyRepo,
    pub error_classification_repo: ErrorClassificationRepo,
    pub payment_verification_repo: PaymentVerificationRepo,
    pub webhook_dispatcher: WebhookDispatcher,
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
        let experiment_ctx = self
            .resolve_experiment(&req, &method, &amount_bucket)
            .await
            .map_err(internal)?;

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
        let mut ranked = apply_experiment_override(ranked, experiment_ctx.as_ref().and_then(|c| c.forced_gateway.clone()));
        let bandit_segment = format!(\"{}:{}\", method, amount_bucket);
        if experiment_ctx.as_ref().and_then(|c| c.forced_gateway.clone()).is_none() {
            ranked = self
                .apply_bandit_if_enabled(&bandit_segment, ranked)
                .await
                .map_err(internal)?;
        }
        if ranked.is_empty() {
            return Err((
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                err("ROUTER_SELECTION_FAILED", "failed to score gateway candidates"),
            ));
        }

        let policy = self
            .retry_policy_repo
            .get_for_merchant(&req.merchant_id)
            .await
            .map_err(internal)?;

        let payment_id = Uuid::new_v4();
        let retry_started = Instant::now();

        let mut final_result: Option<(RankedGateway, GatewayResult, i32, String)> = None;
        let mut pending_verification_gateway: Option<String> = None;

        let max_attempts = attempt_limit(&policy) as usize;
        for (idx, ranked_gateway) in ranked.iter().take(max_attempts).enumerate() {
            if should_stop_for_budget(retry_started, &policy) {
                break;
            }

            let attempt_number = (idx + 1) as i32;
            let Some(selected_gateway) = find_gateway_config(&candidates, &ranked_gateway.gateway_id) else {
                continue;
            };

            let (circuit_allowed, was_probe, circuit_state, circuit_reason) = self
                .check_circuit(&ranked_gateway.gateway_id, &method)
                .await
                .map_err(internal)?;

            if !circuit_allowed {
                self.payment_attempts_repo
                    .insert(NewPaymentAttempt {
                        payment_id,
                        attempt_number,
                        gateway_used: ranked_gateway.gateway_id.clone(),
                        status: "SKIPPED".to_string(),
                        error_code: None,
                        latency_ms: 0,
                        circuit_breaker_state: Some(circuit_state),
                        fallback_reason: Some(circuit_reason),
                        transaction_ref: None,
                    })
                    .await
                    .map_err(internal)?;
                continue;
            }

            let gateway_request = GatewayRequest {
                amount_minor: req.amount_minor,
                currency: req.currency.clone(),
                merchant_id: req.merchant_id.clone(),
            };

            let (gateway_result, latency_ms) = self
                .execute_gateway_call(&selected_gateway, &context, gateway_request)
                .await
                .map_err(internal)?;

            self.payment_attempts_repo
                .insert(NewPaymentAttempt {
                    payment_id,
                    attempt_number,
                    gateway_used: gateway_result.gateway_used.clone(),
                    status: format!("{:?}", gateway_result.response.status).to_uppercase(),
                    error_code: gateway_result.response.error_code.clone(),
                    latency_ms,
                    circuit_breaker_state: Some(circuit_state.clone()),
                    fallback_reason: if attempt_number == 1 {
                        None
                    } else {
                        Some("fallback_retry".to_string())
                    },
                    transaction_ref: gateway_result.response.transaction_id.clone(),
                })
                .await
                .map_err(internal)?;

            self.update_circuit_state(
                &ranked_gateway.gateway_id,
                &method,
                &format!("{:?}", gateway_result.response.status).to_uppercase(),
                was_probe,
            )
            .await
            .map_err(internal)?;

            let error_class = if let Some(code) = &gateway_result.response.error_code {
                Some(
                    self.error_classification_repo
                        .classify(&ranked_gateway.gateway_id, code)
                        .await
                        .map_err(internal)?,
                )
            } else {
                None
            };

            match classify_attempt_result(
                &gateway_result.response.status,
                error_class.as_ref(),
                policy.retry_on_timeout,
            ) {
                RetryDirective::Success => {
                    final_result = Some((
                        ranked_gateway.clone(),
                        gateway_result,
                        latency_ms,
                        if attempt_number == 1 {
                            "primary_success".to_string()
                        } else {
                            format!("fallback_success_attempt_{}", attempt_number)
                        },
                    ));
                    break;
                }
                RetryDirective::PendingVerification => {
                    pending_verification_gateway = Some(ranked_gateway.gateway_id.clone());
                    final_result = Some((
                        ranked_gateway.clone(),
                        GatewayResult {
                            gateway_used: ranked_gateway.gateway_id.clone(),
                            response: NormalizedGatewayResponse {
                                status: PaymentStatus::PendingVerification,
                                transaction_id: gateway_result.response.transaction_id.clone(),
                                auth_code: None,
                                error_code: gateway_result.response.error_code.clone(),
                                error_message: gateway_result.response.error_message.clone(),
                                gateway_response_code: gateway_result.response.gateway_response_code.clone(),
                            },
                        },
                        latency_ms,
                        "timeout_pending_verification".to_string(),
                    ));
                    break;
                }
                RetryDirective::Continue => continue,
                RetryDirective::FailNow => {
                    final_result = Some((
                        ranked_gateway.clone(),
                        gateway_result,
                        latency_ms,
                        "non_retryable_failure".to_string(),
                    ));
                    break;
                }
            }
        }

        let Some((selected, gateway_result, latency_ms, outcome_reason)) = final_result else {
            return Err((
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                err("RETRY_EXHAUSTED", "no gateway could complete payment within retry budget"),
            ));
        };

        if let Some(gateway_id) = &pending_verification_gateway {
            self.payment_verification_repo
                .enqueue_timeout(
                    payment_id,
                    gateway_id,
                    chrono::Utc::now() + chrono::Duration::minutes(2),
                )
                .await
                .map_err(internal)?;
        }

        let routing_reason = format!(
            "reason={}, top_score={:.4}, runner_up={}, experiment={}",
            outcome_reason,
            selected.score,
            ranked
                .get(1)
                .map(|r| r.gateway_id.clone())
                .unwrap_or_else(|| "none".to_string()),
            experiment_ctx
                .as_ref()
                .map(|e| format!("{}:{}", e.experiment_id, e.variant))
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
            routing_strategy: "SCORING_ENGINE_FALLBACK".to_string(),
            routing_reason: routing_reason.clone(),
            status: gateway_result.response.status.clone(),
            gateway_transaction_ref: gateway_result.response.transaction_id.clone(),
            gateway_response_code: gateway_result.response.gateway_response_code.clone(),
            error_message: gateway_result.response.error_message.clone(),
            latency_ms: retry_started.elapsed().as_millis() as i32,
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
                "SCORING_ENGINE_FALLBACK",
                &routing_reason,
                serde_json::to_value(&selected.breakdown).map_err(|e| internal(e.into()))?,
                serde_json::to_value(&ranked).map_err(|e| internal(e.into()))?,
            )
            .await
            .map_err(internal)?;

        if let Some(exp) = &experiment_ctx {
            self.experiments_repo
                .record_result(
                    exp.experiment_id,
                    &exp.variant,
                    hour_floor(chrono::Utc::now()),
                    matches!(gateway_result.response.status, PaymentStatus::Success),
                    latency_ms,
                    req.amount_minor,
                )
                .await
                .map_err(internal)?;
        }

        let _ = self
            .bandit_repo
            .update_outcome(
                &bandit_segment,
                &selected.gateway_id,
                matches!(gateway_result.response.status, PaymentStatus::Success),
            )
            .await;

        Ok(CreatePaymentResponse {
            payment_id,
            status: gateway_result.response.status,
            gateway_used: gateway_result.gateway_used,
            transaction_ref: gateway_result.response.transaction_id,
            routing_strategy: "SCORING_ENGINE_FALLBACK".to_string(),
            routing_reason,
            latency_ms: retry_started.elapsed().as_millis() as i32,
        })
    }

    async fn check_circuit(
        &self,
        gateway_id: &str,
        method: &str,
    ) -> anyhow::Result<(bool, bool, String, String)> {
        let override_state = self.circuit_store.get_override(gateway_id, method).await?;
        if override_state.as_deref() == Some("FORCE_OPEN") {
            return Ok((false, false, "OPEN".to_string(), "manual_force_open".to_string()));
        }
        if override_state.as_deref() == Some("FORCE_CLOSED") {
            return Ok((true, false, "CLOSED".to_string(), "manual_force_closed".to_string()));
        }

        let thresholds = self
            .circuit_breaker_config_repo
            .get_thresholds(gateway_id, method)
            .await?;
        let snapshot = self.circuit_store.get_snapshot(gateway_id, method).await?;

        match pre_call_decision(&snapshot, &thresholds, chrono::Utc::now()) {
            CircuitDecision::Allow => Ok((true, false, format!("{:?}", snapshot.state).to_uppercase(), "allow".to_string())),
            CircuitDecision::Probe => Ok((true, true, "HALF_OPEN".to_string(), "probe".to_string())),
            CircuitDecision::Reject(reason) => Ok((false, false, format!("{:?}", snapshot.state).to_uppercase(), reason)),
        }
    }

    async fn execute_gateway_call(
        &self,
        gateway: &GatewayConfig,
        context: &crate::domain::context::PaymentContext,
        gateway_request: GatewayRequest,
    ) -> anyhow::Result<(GatewayResult, i32)> {
        let timeout_ms = gateway.timeout_ms.max(100) as u64;
        let started = Instant::now();

        let call_future = async {
            if gateway.adapter_type == "RAZORPAY" {
                self.razorpay.initiate_payment(context, gateway_request).await
            } else {
                let mock = MockGateway {
                    gateway_name: gateway.gateway_id.clone(),
                    behavior: gateway
                        .mock_behavior
                        .clone()
                        .unwrap_or_else(|| "ALWAYS_SUCCESS".to_string()),
                };
                mock.initiate_payment(context, gateway_request).await
            }
        };

        let result = tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), call_future).await;
        let latency = started.elapsed().as_millis() as i32;

        let gateway_result = match result {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => GatewayResult {
                gateway_used: gateway.gateway_id.clone(),
                response: NormalizedGatewayResponse {
                    status: PaymentStatus::Failure,
                    transaction_id: None,
                    auth_code: None,
                    error_code: Some("NETWORK_ERROR".to_string()),
                    error_message: Some(e.to_string()),
                    gateway_response_code: None,
                },
            },
            Err(_) => GatewayResult {
                gateway_used: gateway.gateway_id.clone(),
                response: NormalizedGatewayResponse {
                    status: PaymentStatus::Timeout,
                    transaction_id: None,
                    auth_code: None,
                    error_code: Some("GATEWAY_TIMEOUT".to_string()),
                    error_message: Some("gateway timed out".to_string()),
                    gateway_response_code: Some("504".to_string()),
                },
            },
        };

        Ok((gateway_result, latency))
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

        let prev_state = format!("{:?}", snapshot.state);
        let updated = apply_transition(
            snapshot,
            &thresholds,
            failure_rate_2m,
            timeout_rate_5m,
            status,
            was_probe,
            now,
        );

        let next_state = format!("{:?}", updated.state);
        if next_state != prev_state {
            let event_type = if format!("{:?}", updated.state) == "Open" {
                "circuit.opened"
            } else if format!("{:?}", updated.state) == "Closed" {
                "circuit.closed"
            } else {
                "circuit.half_open"
            };
            let _ = self
                .webhook_dispatcher
                .emit(
                    event_type,
                    serde_json::json!({
                        "gateway_id": gateway_id,
                        "method": method,
                        "from_state": prev_state,
                        "to_state": next_state,
                        "failure_rate_2m": updated.failure_rate_2m,
                        "timeout_rate_5m": updated.timeout_rate_5m
                    }),
                )
                .await;
        }

        self.circuit_store.save_snapshot(&updated).await?;
        Ok(())
    }

    async fn resolve_issuing_bank(
        &self,
        req: &CreatePaymentRequest,
        context: &crate::domain::context::PaymentContext,
    ) -> anyhow::Result<String> {
        if let PaymentInstrument::Card(card) = &req.instrument {
            if let Some(bank) = self
                .scoring_config_repo
                .resolve_bank_from_bin(&card.number)
                .await?
            {
                return Ok(bank.to_uppercase());
            }
        }

        Ok(context
            .issuing_bank
            .clone()
            .unwrap_or_else(|| "UNKNOWN".to_string())
            .to_uppercase())
    }

    async fn resolve_experiment(
        &self,
        req: &CreatePaymentRequest,
        method: &str,
        amount_bucket: &str,
    ) -> anyhow::Result<Option<ResolvedExperiment>> {
        let active = self.experiments_repo.get_active_with_filters().await?;
        let input = ExperimentMatchInput {
            payment_method: method.to_string(),
            amount_minor: req.amount_minor,
            merchant_id: req.merchant_id.clone(),
            amount_bucket: amount_bucket.to_string(),
        };

        for (exp, filter) in active {
            if !experiment_matches(&filter, &input) {
                continue;
            }

            let assignment = assign_variant(&req.customer_id, exp.experiment_id, exp.traffic_control_pct);
            self.experiments_repo
                .upsert_assignment(exp.experiment_id, &req.customer_id, &assignment.variant, assignment.bucket)
                .await?;

            let forced_gateway = if assignment.variant == "treatment" {
                Some(exp.treatment_gateway.clone())
            } else {
                None
            };

            return Ok(Some(ResolvedExperiment {
                experiment_id: exp.experiment_id,
                variant: assignment.variant,
                forced_gateway,
            }));
        }

        Ok(None)
    }

    async fn apply_bandit_if_enabled(
        &self,
        segment: &str,
        ranked: Vec<RankedGateway>,
    ) -> anyhow::Result<Vec<RankedGateway>> {
        if !self.bandit_repo.is_enabled(segment).await? {
            return Ok(ranked);
        }

        let gateway_ids: Vec<String> = ranked.iter().map(|r| r.gateway_id.clone()).collect();
        let sampled = self.bandit_repo.sample_scores(segment, &gateway_ids).await?;
        let mut score_map = std::collections::HashMap::new();
        for (gateway, score) in sampled {
            score_map.insert(gateway, score);
        }

        let mut reordered = ranked;
        reordered.sort_by(|a, b| {
            let sa = score_map.get(&a.gateway_id).copied().unwrap_or(0.0);
            let sb = score_map.get(&b.gateway_id).copied().unwrap_or(0.0);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(reordered)
    }
}

#[derive(Debug, Clone)]
struct ResolvedExperiment {
    experiment_id: uuid::Uuid,
    variant: String,
    forced_gateway: Option<String>,
}

fn apply_experiment_override(mut ranked: Vec<RankedGateway>, forced_gateway: Option<String>) -> Vec<RankedGateway> {
    if let Some(forced_gateway) = forced_gateway {
        if let Some(index) = ranked.iter().position(|r| r.gateway_id == forced_gateway) {
            let forced = ranked.remove(index);
            ranked.insert(0, forced);
        }
    }
    ranked
}

fn hour_floor(ts: chrono::DateTime<chrono::Utc>) -> chrono::DateTime<chrono::Utc> {
    let secs = ts.timestamp() - (ts.minute() as i64 * 60) - ts.second() as i64;
    chrono::DateTime::<chrono::Utc>::from_timestamp(secs, 0).unwrap_or(ts)
}

fn find_gateway_config(candidates: &[GatewayCandidate], gateway_id: &str) -> Option<GatewayConfig> {
    candidates
        .iter()
        .find(|c| c.gateway.gateway_id == gateway_id)
        .map(|c| c.gateway.clone())
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
            err("INVALID_CURRENCY", "only INR supported"),
        ));
    }
    if req.customer_id.trim().is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            err("INVALID_CUSTOMER_ID", "customer_id is required"),
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
        "PENDING_VERIFICATION" => PaymentStatus::PendingVerification,
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

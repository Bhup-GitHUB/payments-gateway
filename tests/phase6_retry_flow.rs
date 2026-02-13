use payments_gateway::domain::payment::PaymentStatus;
use payments_gateway::repo::error_classification_repo::ErrorClass;
use payments_gateway::repo::retry_policy_repo::RetryPolicy;
use payments_gateway::service::retry_orchestrator::{attempt_limit, classify_attempt_result, should_stop_for_budget, RetryDirective};

#[test]
fn retryable_failure_continues() {
    let class = ErrorClass {
        retryable: true,
        timeout_like: false,
        non_retryable_user_error: false,
    };
    let directive = classify_attempt_result(&PaymentStatus::Failure, Some(&class), false);
    assert!(matches!(directive, RetryDirective::Continue));
}

#[test]
fn timeout_becomes_pending_verification_by_default() {
    let directive = classify_attempt_result(&PaymentStatus::Timeout, None, false);
    assert!(matches!(directive, RetryDirective::PendingVerification));
}

#[test]
fn attempt_limit_respects_policy() {
    let p = RetryPolicy {
        merchant_id: "m1".to_string(),
        max_attempts: 3,
        latency_budget_ms: 10_000,
        retry_on_timeout: false,
        enabled: true,
    };
    assert_eq!(attempt_limit(&p), 3);
}

#[test]
fn budget_cutoff_detected() {
    let p = RetryPolicy {
        merchant_id: "m1".to_string(),
        max_attempts: 3,
        latency_budget_ms: 0,
        retry_on_timeout: false,
        enabled: true,
    };
    let start = std::time::Instant::now();
    assert!(should_stop_for_budget(start, &p));
}

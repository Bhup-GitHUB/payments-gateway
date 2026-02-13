use payments_gateway::domain::experiment::{ExperimentFilter, ExperimentResultRow};
use payments_gateway::experiments::analyzer::analyze;
use payments_gateway::experiments::assigner::assign_variant;
use payments_gateway::experiments::filter::{matches, MatchInput};
use uuid::Uuid;

#[test]
fn assignment_is_deterministic() {
    let id = Uuid::new_v4();
    let a = assign_variant("cust-1", id, 90);
    let b = assign_variant("cust-1", id, 90);
    assert_eq!(a.variant, b.variant);
    assert_eq!(a.bucket, b.bucket);
}

#[test]
fn filter_matching_works() {
    let filter = ExperimentFilter {
        experiment_id: Uuid::new_v4(),
        payment_method: Some("UPI".to_string()),
        min_amount_minor: Some(1000),
        max_amount_minor: Some(10_000),
        merchant_id: Some("m1".to_string()),
        amount_bucket: Some("500_2000".to_string()),
    };

    let input = MatchInput {
        payment_method: "UPI".to_string(),
        amount_minor: 5000,
        merchant_id: "m1".to_string(),
        amount_bucket: "500_2000".to_string(),
    };

    assert!(matches(&filter, &input));
}

#[test]
fn z_test_returns_winner_for_clear_gap() {
    let experiment_id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let rows = vec![
        ExperimentResultRow {
            experiment_id,
            variant: "control".to_string(),
            date_hour: now,
            total_requests: 1000,
            successful_requests: 900,
            failed_requests: 100,
            avg_latency_ms: 1800,
            p95_latency_ms: 2200,
            total_revenue_minor: 100,
        },
        ExperimentResultRow {
            experiment_id,
            variant: "treatment".to_string(),
            date_hour: now,
            total_requests: 1000,
            successful_requests: 970,
            failed_requests: 30,
            avg_latency_ms: 1700,
            p95_latency_ms: 2100,
            total_revenue_minor: 100,
        },
    ];

    let out = analyze(&rows, 100);
    assert!(out.is_significant);
    assert_eq!(out.winner.as_deref(), Some("treatment"));
}

use payments_gateway::circuit::state::{CircuitSnapshot, CircuitState};
use payments_gateway::circuit::transitions::apply_transition;
use payments_gateway::repo::circuit_breaker_config_repo::CircuitThresholds;

#[test]
fn opens_when_failure_rate_crosses_threshold() {
    let snapshot = CircuitSnapshot::new("g1", "UPI");
    let thresholds = defaults();
    let now = chrono::Utc::now();

    let out = apply_transition(snapshot, &thresholds, 0.5, 0.1, "FAILURE", false, now);
    assert_eq!(out.state, CircuitState::Open);
}

#[test]
fn closes_half_open_on_success_streak() {
    let mut snapshot = CircuitSnapshot::new("g1", "UPI");
    snapshot.state = CircuitState::HalfOpen;
    snapshot.success_streak = 4;

    let thresholds = defaults();
    let now = chrono::Utc::now();

    let out = apply_transition(snapshot, &thresholds, 0.1, 0.1, "SUCCESS", true, now);
    assert_eq!(out.state, CircuitState::Closed);
}

fn defaults() -> CircuitThresholds {
    CircuitThresholds {
        failure_rate_threshold_2m: 0.40,
        consecutive_failure_threshold: 10,
        timeout_rate_threshold_5m: 0.50,
        cooldown_seconds: 30,
        half_open_probe_ratio: 0.10,
        half_open_min_probe_count: 5,
        half_open_success_rate_close: 0.80,
        half_open_consecutive_success_close: 5,
        half_open_consecutive_failure_reopen: 3,
    }
}

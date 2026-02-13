use crate::circuit::state::{CircuitDecision, CircuitSnapshot, CircuitState};
use crate::repo::circuit_breaker_config_repo::CircuitThresholds;

pub fn pre_call_decision(
    snapshot: &CircuitSnapshot,
    thresholds: &CircuitThresholds,
    now: chrono::DateTime<chrono::Utc>,
) -> CircuitDecision {
    match snapshot.state {
        CircuitState::Closed => CircuitDecision::Allow,
        CircuitState::Open => {
            if snapshot.cooldown_until.is_some_and(|t| now >= t) {
                CircuitDecision::Probe
            } else {
                CircuitDecision::Reject("circuit open".to_string())
            }
        }
        CircuitState::HalfOpen => {
            let r: f64 = rand::random();
            if r <= thresholds.half_open_probe_ratio {
                CircuitDecision::Probe
            } else {
                CircuitDecision::Reject("half-open non-probe request".to_string())
            }
        }
    }
}

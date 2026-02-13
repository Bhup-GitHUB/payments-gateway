use crate::circuit::state::{CircuitSnapshot, CircuitState};
use crate::repo::circuit_breaker_config_repo::CircuitThresholds;

pub fn apply_transition(
    mut snapshot: CircuitSnapshot,
    thresholds: &CircuitThresholds,
    failure_rate_2m: f64,
    timeout_rate_5m: f64,
    status: &str,
    was_probe: bool,
    now: chrono::DateTime<chrono::Utc>,
) -> CircuitSnapshot {
    snapshot.failure_rate_2m = failure_rate_2m;
    snapshot.timeout_rate_5m = timeout_rate_5m;

    if status == "SUCCESS" {
        snapshot.consecutive_failures = 0;
        snapshot.success_streak += 1;
        if was_probe {
            snapshot.probe_total += 1;
            snapshot.probe_success += 1;
            snapshot.probe_failure_streak = 0;
        }
    } else {
        snapshot.consecutive_failures += 1;
        snapshot.success_streak = 0;
        if was_probe {
            snapshot.probe_total += 1;
            snapshot.probe_failure_streak += 1;
        }
    }

    match snapshot.state {
        CircuitState::Closed => {
            if failure_rate_2m > thresholds.failure_rate_threshold_2m
                || timeout_rate_5m > thresholds.timeout_rate_threshold_5m
                || snapshot.consecutive_failures >= thresholds.consecutive_failure_threshold
            {
                snapshot.state = CircuitState::Open;
                snapshot.opened_at = Some(now);
                snapshot.cooldown_until = Some(now + chrono::Duration::seconds(thresholds.cooldown_seconds as i64));
                snapshot.probe_total = 0;
                snapshot.probe_success = 0;
                snapshot.probe_failure_streak = 0;
                snapshot.success_streak = 0;
            }
        }
        CircuitState::Open => {
            if snapshot.cooldown_until.is_some_and(|t| now >= t) {
                snapshot.state = CircuitState::HalfOpen;
                snapshot.probe_total = 0;
                snapshot.probe_success = 0;
                snapshot.probe_failure_streak = 0;
                snapshot.success_streak = 0;
            }
        }
        CircuitState::HalfOpen => {
            if snapshot.probe_failure_streak >= thresholds.half_open_consecutive_failure_reopen {
                snapshot.state = CircuitState::Open;
                snapshot.opened_at = Some(now);
                snapshot.cooldown_until = Some(now + chrono::Duration::seconds(thresholds.cooldown_seconds as i64));
                snapshot.probe_total = 0;
                snapshot.probe_success = 0;
                snapshot.probe_failure_streak = 0;
                snapshot.success_streak = 0;
            } else if snapshot.success_streak >= thresholds.half_open_consecutive_success_close {
                snapshot.state = CircuitState::Closed;
                snapshot.probe_total = 0;
                snapshot.probe_success = 0;
                snapshot.probe_failure_streak = 0;
                snapshot.success_streak = 0;
                snapshot.cooldown_until = None;
            } else if snapshot.probe_total >= thresholds.half_open_min_probe_count {
                let ratio = snapshot.probe_success as f64 / snapshot.probe_total as f64;
                if ratio >= thresholds.half_open_success_rate_close {
                    snapshot.state = CircuitState::Closed;
                    snapshot.probe_total = 0;
                    snapshot.probe_success = 0;
                    snapshot.probe_failure_streak = 0;
                    snapshot.success_streak = 0;
                    snapshot.cooldown_until = None;
                }
            }
        }
    }

    snapshot.updated_at = now;
    snapshot
}

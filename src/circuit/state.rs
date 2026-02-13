use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitSnapshot {
    pub gateway_id: String,
    pub payment_method: String,
    pub state: CircuitState,
    pub failure_rate_2m: f64,
    pub timeout_rate_5m: f64,
    pub consecutive_failures: i32,
    pub opened_at: Option<chrono::DateTime<chrono::Utc>>,
    pub cooldown_until: Option<chrono::DateTime<chrono::Utc>>,
    pub probe_total: i32,
    pub probe_success: i32,
    pub probe_failure_streak: i32,
    pub success_streak: i32,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl CircuitSnapshot {
    pub fn new(gateway_id: &str, payment_method: &str) -> Self {
        Self {
            gateway_id: gateway_id.to_string(),
            payment_method: payment_method.to_string(),
            state: CircuitState::Closed,
            failure_rate_2m: 0.0,
            timeout_rate_5m: 0.0,
            consecutive_failures: 0,
            opened_at: None,
            cooldown_until: None,
            probe_total: 0,
            probe_success: 0,
            probe_failure_streak: 0,
            success_streak: 0,
            updated_at: chrono::Utc::now(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum CircuitDecision {
    Allow,
    Probe,
    Reject(String),
}

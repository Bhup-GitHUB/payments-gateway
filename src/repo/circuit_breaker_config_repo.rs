use anyhow::Result;
use sqlx::{PgPool, Row};

#[derive(Clone)]
pub struct CircuitBreakerConfigRepo {
    pub pool: PgPool,
}

#[derive(Debug, Clone)]
pub struct CircuitThresholds {
    pub failure_rate_threshold_2m: f64,
    pub consecutive_failure_threshold: i32,
    pub timeout_rate_threshold_5m: f64,
    pub cooldown_seconds: i32,
    pub half_open_probe_ratio: f64,
    pub half_open_min_probe_count: i32,
    pub half_open_success_rate_close: f64,
    pub half_open_consecutive_success_close: i32,
    pub half_open_consecutive_failure_reopen: i32,
}

impl CircuitBreakerConfigRepo {
    pub async fn get_thresholds(&self, gateway_id: &str, payment_method: &str) -> Result<CircuitThresholds> {
        let row = sqlx::query(
            r#"
            SELECT failure_rate_threshold_2m, consecutive_failure_threshold, timeout_rate_threshold_5m,
                   cooldown_seconds, half_open_probe_ratio, half_open_min_probe_count,
                   half_open_success_rate_close, half_open_consecutive_success_close,
                   half_open_consecutive_failure_reopen
            FROM circuit_breaker_config WHERE gateway_id=$1 AND payment_method=$2
            "#,
        )
        .bind(gateway_id)
        .bind(payment_method)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(CircuitThresholds {
                failure_rate_threshold_2m: row.get("failure_rate_threshold_2m"),
                consecutive_failure_threshold: row.get("consecutive_failure_threshold"),
                timeout_rate_threshold_5m: row.get("timeout_rate_threshold_5m"),
                cooldown_seconds: row.get("cooldown_seconds"),
                half_open_probe_ratio: row.get("half_open_probe_ratio"),
                half_open_min_probe_count: row.get("half_open_min_probe_count"),
                half_open_success_rate_close: row.get("half_open_success_rate_close"),
                half_open_consecutive_success_close: row.get("half_open_consecutive_success_close"),
                half_open_consecutive_failure_reopen: row.get("half_open_consecutive_failure_reopen"),
            })
        } else {
            Ok(CircuitThresholds {
                failure_rate_threshold_2m: 0.40,
                consecutive_failure_threshold: 10,
                timeout_rate_threshold_5m: 0.50,
                cooldown_seconds: 30,
                half_open_probe_ratio: 0.10,
                half_open_min_probe_count: 5,
                half_open_success_rate_close: 0.80,
                half_open_consecutive_success_close: 5,
                half_open_consecutive_failure_reopen: 3,
            })
        }
    }
}

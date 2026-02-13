use anyhow::Result;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct PaymentAttemptsRepo {
    pub pool: PgPool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PaymentAttemptRow {
    pub payment_id: Uuid,
    pub attempt_number: i32,
    pub gateway_used: String,
    pub status: String,
    pub error_code: Option<String>,
    pub latency_ms: i32,
    pub circuit_breaker_state: Option<String>,
    pub fallback_reason: Option<String>,
    pub transaction_ref: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct NewPaymentAttempt {
    pub payment_id: Uuid,
    pub attempt_number: i32,
    pub gateway_used: String,
    pub status: String,
    pub error_code: Option<String>,
    pub latency_ms: i32,
    pub circuit_breaker_state: Option<String>,
    pub fallback_reason: Option<String>,
    pub transaction_ref: Option<String>,
}

impl PaymentAttemptsRepo {
    pub async fn insert(&self, in_row: NewPaymentAttempt) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO payment_attempts (
                payment_id, attempt_number, gateway_used, status, error_code, latency_ms,
                circuit_breaker_state, fallback_reason, transaction_ref
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            ON CONFLICT (payment_id, attempt_number) DO NOTHING
            "#,
        )
        .bind(in_row.payment_id)
        .bind(in_row.attempt_number)
        .bind(in_row.gateway_used)
        .bind(in_row.status)
        .bind(in_row.error_code)
        .bind(in_row.latency_ms)
        .bind(in_row.circuit_breaker_state)
        .bind(in_row.fallback_reason)
        .bind(in_row.transaction_ref)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_by_payment_id(&self, payment_id: Uuid) -> Result<Vec<PaymentAttemptRow>> {
        let rows = sqlx::query(
            r#"
            SELECT payment_id, attempt_number, gateway_used, status, error_code, latency_ms,
                   circuit_breaker_state, fallback_reason, transaction_ref, created_at
            FROM payment_attempts
            WHERE payment_id=$1
            ORDER BY attempt_number ASC
            "#,
        )
        .bind(payment_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| PaymentAttemptRow {
                payment_id: row.get("payment_id"),
                attempt_number: row.get("attempt_number"),
                gateway_used: row.get("gateway_used"),
                status: row.get("status"),
                error_code: row.get("error_code"),
                latency_ms: row.get("latency_ms"),
                circuit_breaker_state: row.get("circuit_breaker_state"),
                fallback_reason: row.get("fallback_reason"),
                transaction_ref: row.get("transaction_ref"),
                created_at: row.get("created_at"),
            })
            .collect())
    }
}

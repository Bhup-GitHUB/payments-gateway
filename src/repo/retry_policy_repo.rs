use anyhow::Result;
use sqlx::{PgPool, Row};

#[derive(Clone)]
pub struct RetryPolicyRepo {
    pub pool: PgPool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RetryPolicy {
    pub merchant_id: String,
    pub max_attempts: i32,
    pub latency_budget_ms: i32,
    pub retry_on_timeout: bool,
    pub enabled: bool,
}

impl RetryPolicyRepo {
    pub async fn get_for_merchant(&self, merchant_id: &str) -> Result<RetryPolicy> {
        let row = sqlx::query(
            "SELECT merchant_id, max_attempts, latency_budget_ms, retry_on_timeout, enabled FROM retry_policy WHERE merchant_id=$1",
        )
        .bind(merchant_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(RetryPolicy {
                merchant_id: row.get("merchant_id"),
                max_attempts: row.get("max_attempts"),
                latency_budget_ms: row.get("latency_budget_ms"),
                retry_on_timeout: row.get("retry_on_timeout"),
                enabled: row.get("enabled"),
            })
        } else {
            Ok(RetryPolicy {
                merchant_id: merchant_id.to_string(),
                max_attempts: 3,
                latency_budget_ms: 10000,
                retry_on_timeout: false,
                enabled: true,
            })
        }
    }

    pub async fn upsert(&self, policy: RetryPolicy) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO retry_policy (merchant_id, max_attempts, latency_budget_ms, retry_on_timeout, enabled, updated_at)
            VALUES ($1,$2,$3,$4,$5,now())
            ON CONFLICT (merchant_id) DO UPDATE SET
                max_attempts=EXCLUDED.max_attempts,
                latency_budget_ms=EXCLUDED.latency_budget_ms,
                retry_on_timeout=EXCLUDED.retry_on_timeout,
                enabled=EXCLUDED.enabled,
                updated_at=now()
            "#,
        )
        .bind(policy.merchant_id)
        .bind(policy.max_attempts)
        .bind(policy.latency_budget_ms)
        .bind(policy.retry_on_timeout)
        .bind(policy.enabled)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

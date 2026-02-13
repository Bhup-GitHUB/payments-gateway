use anyhow::Result;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct PaymentVerificationRepo {
    pub pool: PgPool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct VerificationRow {
    pub payment_id: Uuid,
    pub gateway_id: String,
    pub next_check_at: chrono::DateTime<chrono::Utc>,
    pub attempts: i32,
    pub status: String,
    pub last_response: Option<serde_json::Value>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl PaymentVerificationRepo {
    pub async fn enqueue_timeout(&self, payment_id: Uuid, gateway_id: &str, next_check_at: chrono::DateTime<chrono::Utc>) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO payment_status_verification (payment_id, gateway_id, next_check_at, attempts, status, updated_at)
            VALUES ($1,$2,$3,0,'PENDING',now())
            ON CONFLICT (payment_id) DO UPDATE SET
                gateway_id=EXCLUDED.gateway_id,
                next_check_at=EXCLUDED.next_check_at,
                status='PENDING',
                updated_at=now()
            "#,
        )
        .bind(payment_id)
        .bind(gateway_id)
        .bind(next_check_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn due_items(&self, limit: i64) -> Result<Vec<VerificationRow>> {
        let rows = sqlx::query(
            r#"
            SELECT payment_id, gateway_id, next_check_at, attempts, status, last_response, updated_at
            FROM payment_status_verification
            WHERE status='PENDING' AND next_check_at <= now()
            ORDER BY next_check_at ASC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| VerificationRow {
                payment_id: row.get("payment_id"),
                gateway_id: row.get("gateway_id"),
                next_check_at: row.get("next_check_at"),
                attempts: row.get("attempts"),
                status: row.get("status"),
                last_response: row.get("last_response"),
                updated_at: row.get("updated_at"),
            })
            .collect())
    }

    pub async fn mark(&self, payment_id: Uuid, status: &str, attempts: i32, last_response: serde_json::Value, next_check_at: Option<chrono::DateTime<chrono::Utc>>) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE payment_status_verification
            SET status=$2, attempts=$3, last_response=$4, next_check_at=COALESCE($5, next_check_at), updated_at=now()
            WHERE payment_id=$1
            "#,
        )
        .bind(payment_id)
        .bind(status)
        .bind(attempts)
        .bind(last_response)
        .bind(next_check_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_by_payment_id(&self, payment_id: Uuid) -> Result<Option<VerificationRow>> {
        let row = sqlx::query(
            "SELECT payment_id, gateway_id, next_check_at, attempts, status, last_response, updated_at FROM payment_status_verification WHERE payment_id=$1",
        )
        .bind(payment_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| VerificationRow {
            payment_id: row.get("payment_id"),
            gateway_id: row.get("gateway_id"),
            next_check_at: row.get("next_check_at"),
            attempts: row.get("attempts"),
            status: row.get("status"),
            last_response: row.get("last_response"),
            updated_at: row.get("updated_at"),
        }))
    }
}

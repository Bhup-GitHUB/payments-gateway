use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxEvent {
    pub id: i64,
    pub payment_id: Uuid,
    pub event_type: String,
    pub payload_json: serde_json::Value,
    pub attempts: i32,
}

#[derive(Clone)]
pub struct OutboxRepo {
    pub pool: PgPool,
}

impl OutboxRepo {
    pub async fn insert_tx(
        tx: &mut Transaction<'_, Postgres>,
        payment_id: Uuid,
        event_type: &str,
        payload_json: serde_json::Value,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO payment_events_outbox (payment_id, event_type, payload_json, status, attempts, next_attempt_at)
            VALUES ($1, $2, $3, 'PENDING', 0, now())
            ON CONFLICT (payment_id, event_type) DO NOTHING
            "#,
        )
        .bind(payment_id)
        .bind(event_type)
        .bind(payload_json)
        .execute(tx.as_mut())
        .await?;

        Ok(())
    }

    pub async fn lock_pending(&self, batch_size: i64) -> Result<Vec<OutboxEvent>> {
        let mut tx = self.pool.begin().await?;
        let rows = sqlx::query(
            r#"
            SELECT id, payment_id, event_type, payload_json, attempts
            FROM payment_events_outbox
            WHERE status = 'PENDING' AND next_attempt_at <= now()
            ORDER BY id ASC
            LIMIT $1
            FOR UPDATE SKIP LOCKED
            "#,
        )
        .bind(batch_size)
        .fetch_all(tx.as_mut())
        .await?;

        if rows.is_empty() {
            tx.rollback().await?;
            return Ok(Vec::new());
        }

        let ids: Vec<i64> = rows.iter().map(|r| r.get("id")).collect();
        sqlx::query("UPDATE payment_events_outbox SET status = 'PROCESSING', updated_at = now() WHERE id = ANY($1)")
            .bind(&ids)
            .execute(tx.as_mut())
            .await?;

        tx.commit().await?;

        Ok(rows
            .into_iter()
            .map(|r| OutboxEvent {
                id: r.get("id"),
                payment_id: r.get("payment_id"),
                event_type: r.get("event_type"),
                payload_json: r.get("payload_json"),
                attempts: r.get("attempts"),
            })
            .collect())
    }

    pub async fn mark_published(&self, id: i64) -> Result<()> {
        sqlx::query(
            "UPDATE payment_events_outbox SET status='PUBLISHED', published_at=now(), updated_at=now() WHERE id=$1",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_retry(&self, id: i64, attempts: i32, next_attempt_at: DateTime<Utc>) -> Result<()> {
        sqlx::query(
            "UPDATE payment_events_outbox SET status='PENDING', attempts=$2, next_attempt_at=$3, updated_at=now() WHERE id=$1",
        )
        .bind(id)
        .bind(attempts)
        .bind(next_attempt_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

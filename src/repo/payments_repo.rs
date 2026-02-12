use crate::domain::payment::{CreatePaymentRequest, PaymentStatus};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

pub struct PaymentRecordInput {
    pub payment_id: Uuid,
    pub merchant_id: String,
    pub idempotency_key: String,
    pub request_hash: String,
    pub req: CreatePaymentRequest,
    pub issuing_bank: Option<String>,
    pub gateway_used: String,
    pub routing_strategy: String,
    pub routing_reason: String,
    pub status: PaymentStatus,
    pub gateway_transaction_ref: Option<String>,
    pub gateway_response_code: Option<String>,
    pub error_message: Option<String>,
    pub latency_ms: i32,
}

#[derive(Clone)]
pub struct PaymentsRepo {
    pub pool: PgPool,
}

#[derive(Debug, Clone)]
pub struct StoredPayment {
    pub payment_id: Uuid,
    pub status: String,
    pub gateway_used: String,
    pub gateway_transaction_ref: Option<String>,
    pub routing_strategy: String,
    pub routing_reason: String,
    pub latency_ms: i32,
    pub request_hash: String,
}

impl PaymentsRepo {
    pub async fn find_by_idempotency(
        &self,
        merchant_id: &str,
        idempotency_key: &str,
    ) -> anyhow::Result<Option<StoredPayment>> {
        let row = sqlx::query(
            r#"
            SELECT payment_id, status, gateway_used, gateway_transaction_ref, routing_strategy, routing_reason, latency_ms, request_hash
            FROM payments
            WHERE merchant_id = $1 AND idempotency_key = $2
            "#,
        )
        .bind(merchant_id)
        .bind(idempotency_key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| StoredPayment {
            payment_id: r.get("payment_id"),
            status: r.get("status"),
            gateway_used: r.get("gateway_used"),
            gateway_transaction_ref: r.get("gateway_transaction_ref"),
            routing_strategy: r.get("routing_strategy"),
            routing_reason: r.get("routing_reason"),
            latency_ms: r.get("latency_ms"),
            request_hash: r.get("request_hash"),
        }))
    }

    pub async fn insert_payment_tx(
        tx: &mut Transaction<'_, Postgres>,
        data: &PaymentRecordInput,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO payments (
                payment_id, merchant_id, idempotency_key, request_hash, amount_minor, currency,
                payment_method, issuing_bank, gateway_used, routing_strategy, routing_reason,
                status, gateway_transaction_ref, gateway_response_code, error_message, latency_ms
            ) VALUES (
                $1, $2, $3, $4, $5, $6,
                $7, $8, $9, $10, $11,
                $12, $13, $14, $15, $16
            )
            "#,
        )
        .bind(data.payment_id)
        .bind(data.merchant_id)
        .bind(data.idempotency_key)
        .bind(data.request_hash)
        .bind(data.req.amount_minor)
        .bind(data.req.currency.clone())
        .bind(format!("{:?}", data.req.payment_method))
        .bind(data.issuing_bank.clone())
        .bind(data.gateway_used.clone())
        .bind(data.routing_strategy.clone())
        .bind(data.routing_reason.clone())
        .bind(format!("{:?}", data.status).to_uppercase())
        .bind(data.gateway_transaction_ref.clone())
        .bind(data.gateway_response_code.clone())
        .bind(data.error_message.clone())
        .bind(data.latency_ms)
        .execute(tx.as_mut())
        .await?;

        Ok(())
    }
}

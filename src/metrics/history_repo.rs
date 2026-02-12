use crate::metrics::aggregator::{AggregatedMetric, MetricKey};
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

#[derive(Clone)]
pub struct MetricsHistoryRepo {
    pub pool: PgPool,
}

impl MetricsHistoryRepo {
    pub async fn insert_snapshot(
        &self,
        snapshot_minute: DateTime<Utc>,
        key: &MetricKey,
        window_size_minutes: i32,
        metric: &AggregatedMetric,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO gateway_metrics (
                snapshot_minute,
                gateway_name,
                payment_method,
                issuing_bank,
                window_size_minutes,
                success_rate,
                timeout_rate,
                avg_latency_ms,
                p50_latency_ms,
                p95_latency_ms,
                p99_latency_ms,
                total_requests,
                failed_requests,
                timeout_requests,
                error_counts_json
            ) VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15
            )
            ON CONFLICT (snapshot_minute, gateway_name, payment_method, issuing_bank, window_size_minutes)
            DO UPDATE SET
              success_rate = EXCLUDED.success_rate,
              timeout_rate = EXCLUDED.timeout_rate,
              avg_latency_ms = EXCLUDED.avg_latency_ms,
              p50_latency_ms = EXCLUDED.p50_latency_ms,
              p95_latency_ms = EXCLUDED.p95_latency_ms,
              p99_latency_ms = EXCLUDED.p99_latency_ms,
              total_requests = EXCLUDED.total_requests,
              failed_requests = EXCLUDED.failed_requests,
              timeout_requests = EXCLUDED.timeout_requests,
              error_counts_json = EXCLUDED.error_counts_json
            "#,
        )
        .bind(snapshot_minute)
        .bind(&key.gateway)
        .bind(&key.method)
        .bind(&key.bank)
        .bind(window_size_minutes)
        .bind(metric.success_rate)
        .bind(metric.timeout_rate)
        .bind(metric.avg_latency_ms)
        .bind(metric.p50_latency_ms)
        .bind(metric.p95_latency_ms)
        .bind(metric.p99_latency_ms)
        .bind(metric.total_requests as i64)
        .bind(metric.failed_requests as i64)
        .bind(metric.timeout_requests as i64)
        .bind(serde_json::to_value(&metric.error_counts)?)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

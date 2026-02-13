use crate::domain::experiment::{Experiment, ExperimentFilter, ExperimentResultRow};
use anyhow::Result;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct ExperimentsRepo {
    pub pool: PgPool,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct CreateExperimentInput {
    pub name: String,
    pub traffic_control_pct: i32,
    pub traffic_treatment_pct: i32,
    pub treatment_gateway: String,
    pub start_date: chrono::DateTime<chrono::Utc>,
    pub end_date: Option<chrono::DateTime<chrono::Utc>>,
    pub created_by: String,
    pub filter: CreateExperimentFilterInput,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct CreateExperimentFilterInput {
    pub payment_method: Option<String>,
    pub min_amount_minor: Option<i64>,
    pub max_amount_minor: Option<i64>,
    pub merchant_id: Option<String>,
    pub amount_bucket: Option<String>,
}

impl ExperimentsRepo {
    pub async fn create(&self, input: CreateExperimentInput) -> Result<Experiment> {
        let experiment_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO experiments (
                experiment_id, name, status, traffic_control_pct, traffic_treatment_pct,
                treatment_gateway, start_date, end_date, created_by
            ) VALUES ($1,$2,'RUNNING',$3,$4,$5,$6,$7,$8)
            "#,
        )
        .bind(experiment_id)
        .bind(&input.name)
        .bind(input.traffic_control_pct)
        .bind(input.traffic_treatment_pct)
        .bind(&input.treatment_gateway)
        .bind(input.start_date)
        .bind(input.end_date)
        .bind(&input.created_by)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO experiment_filters (
                experiment_id, payment_method, min_amount_minor, max_amount_minor, merchant_id, amount_bucket
            ) VALUES ($1,$2,$3,$4,$5,$6)
            "#,
        )
        .bind(experiment_id)
        .bind(input.filter.payment_method)
        .bind(input.filter.min_amount_minor)
        .bind(input.filter.max_amount_minor)
        .bind(input.filter.merchant_id)
        .bind(input.filter.amount_bucket)
        .execute(&self.pool)
        .await?;

        Ok(Experiment {
            experiment_id,
            name: input.name,
            status: "RUNNING".to_string(),
            traffic_control_pct: input.traffic_control_pct,
            traffic_treatment_pct: input.traffic_treatment_pct,
            treatment_gateway: input.treatment_gateway,
            start_date: input.start_date,
            end_date: input.end_date,
            created_by: input.created_by,
        })
    }

    pub async fn list(&self) -> Result<Vec<Experiment>> {
        let rows = sqlx::query(
            "SELECT experiment_id, name, status, traffic_control_pct, traffic_treatment_pct, treatment_gateway, start_date, end_date, created_by FROM experiments ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| Experiment {
                experiment_id: row.get("experiment_id"),
                name: row.get("name"),
                status: row.get("status"),
                traffic_control_pct: row.get("traffic_control_pct"),
                traffic_treatment_pct: row.get("traffic_treatment_pct"),
                treatment_gateway: row.get("treatment_gateway"),
                start_date: row.get("start_date"),
                end_date: row.get("end_date"),
                created_by: row.get("created_by"),
            })
            .collect())
    }

    pub async fn get_active_with_filters(&self) -> Result<Vec<(Experiment, ExperimentFilter)>> {
        let rows = sqlx::query(
            r#"
            SELECT e.experiment_id, e.name, e.status, e.traffic_control_pct, e.traffic_treatment_pct,
                   e.treatment_gateway, e.start_date, e.end_date, e.created_by,
                   f.payment_method, f.min_amount_minor, f.max_amount_minor, f.merchant_id, f.amount_bucket
            FROM experiments e
            JOIN experiment_filters f ON e.experiment_id = f.experiment_id
            WHERE e.status='RUNNING' AND e.start_date <= now() AND (e.end_date IS NULL OR e.end_date >= now())
            ORDER BY e.created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                (
                    Experiment {
                        experiment_id: row.get("experiment_id"),
                        name: row.get("name"),
                        status: row.get("status"),
                        traffic_control_pct: row.get("traffic_control_pct"),
                        traffic_treatment_pct: row.get("traffic_treatment_pct"),
                        treatment_gateway: row.get("treatment_gateway"),
                        start_date: row.get("start_date"),
                        end_date: row.get("end_date"),
                        created_by: row.get("created_by"),
                    },
                    ExperimentFilter {
                        experiment_id: row.get("experiment_id"),
                        payment_method: row.get("payment_method"),
                        min_amount_minor: row.get("min_amount_minor"),
                        max_amount_minor: row.get("max_amount_minor"),
                        merchant_id: row.get("merchant_id"),
                        amount_bucket: row.get("amount_bucket"),
                    },
                )
            })
            .collect())
    }

    pub async fn stop(&self, experiment_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE experiments SET status='PAUSED' WHERE experiment_id=$1")
            .bind(experiment_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn upsert_assignment(&self, experiment_id: Uuid, customer_id: &str, variant: &str, bucket: i32) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO experiment_assignments (experiment_id, customer_id, variant, bucket)
            VALUES ($1,$2,$3,$4)
            ON CONFLICT (experiment_id, customer_id) DO UPDATE SET
              variant=EXCLUDED.variant,
              bucket=EXCLUDED.bucket,
              assigned_at=now()
            "#,
        )
        .bind(experiment_id)
        .bind(customer_id)
        .bind(variant)
        .bind(bucket)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn record_result(
        &self,
        experiment_id: Uuid,
        variant: &str,
        date_hour: chrono::DateTime<chrono::Utc>,
        success: bool,
        latency_ms: i32,
        revenue_minor: i64,
    ) -> Result<()> {
        let success_inc = if success { 1_i64 } else { 0_i64 };
        let failed_inc = if success { 0_i64 } else { 1_i64 };

        sqlx::query(
            r#"
            INSERT INTO experiment_results (
                experiment_id, variant, date_hour, total_requests, successful_requests, failed_requests,
                avg_latency_ms, p95_latency_ms, total_revenue_minor
            ) VALUES ($1,$2,$3,1,$4,$5,$6,$6,$7)
            ON CONFLICT (experiment_id, variant, date_hour)
            DO UPDATE SET
                total_requests = experiment_results.total_requests + 1,
                successful_requests = experiment_results.successful_requests + $4,
                failed_requests = experiment_results.failed_requests + $5,
                avg_latency_ms = ((experiment_results.avg_latency_ms * experiment_results.total_requests::int) + $6) / (experiment_results.total_requests::int + 1),
                p95_latency_ms = GREATEST(experiment_results.p95_latency_ms, $6),
                total_revenue_minor = experiment_results.total_revenue_minor + $7
            "#,
        )
        .bind(experiment_id)
        .bind(variant)
        .bind(date_hour)
        .bind(success_inc)
        .bind(failed_inc)
        .bind(latency_ms)
        .bind(revenue_minor)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn results(&self, experiment_id: Uuid) -> Result<Vec<ExperimentResultRow>> {
        let rows = sqlx::query(
            r#"
            SELECT experiment_id, variant, date_hour, total_requests, successful_requests, failed_requests,
                   avg_latency_ms, p95_latency_ms, total_revenue_minor
            FROM experiment_results
            WHERE experiment_id=$1
            ORDER BY date_hour DESC, variant ASC
            "#,
        )
        .bind(experiment_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| ExperimentResultRow {
                experiment_id: row.get("experiment_id"),
                variant: row.get("variant"),
                date_hour: row.get("date_hour"),
                total_requests: row.get("total_requests"),
                successful_requests: row.get("successful_requests"),
                failed_requests: row.get("failed_requests"),
                avg_latency_ms: row.get("avg_latency_ms"),
                p95_latency_ms: row.get("p95_latency_ms"),
                total_revenue_minor: row.get("total_revenue_minor"),
            })
            .collect())
    }
}

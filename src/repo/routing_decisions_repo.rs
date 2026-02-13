use crate::domain::routing_decision::RoutingDecisionRecord;
use anyhow::Result;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct RoutingDecisionsRepo {
    pub pool: PgPool,
}

impl RoutingDecisionsRepo {
    pub async fn insert(
        &self,
        payment_id: Uuid,
        selected_gateway: &str,
        selected_score: f64,
        runner_up_gateway: Option<&str>,
        runner_up_score: Option<f64>,
        strategy: &str,
        reason_summary: &str,
        score_breakdown_json: serde_json::Value,
        ranked_gateways_json: serde_json::Value,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO routing_decisions (
                payment_id,
                selected_gateway,
                selected_score,
                runner_up_gateway,
                runner_up_score,
                strategy,
                reason_summary,
                score_breakdown_json,
                ranked_gateways_json
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            ON CONFLICT (payment_id) DO NOTHING
            "#,
        )
        .bind(payment_id)
        .bind(selected_gateway)
        .bind(selected_score)
        .bind(runner_up_gateway)
        .bind(runner_up_score)
        .bind(strategy)
        .bind(reason_summary)
        .bind(score_breakdown_json)
        .bind(ranked_gateways_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_by_payment_id(&self, payment_id: Uuid) -> Result<Option<RoutingDecisionRecord>> {
        let row = sqlx::query(
            r#"
            SELECT payment_id, selected_gateway, selected_score, runner_up_gateway, runner_up_score,
                   strategy, reason_summary, score_breakdown_json, ranked_gateways_json, created_at
            FROM routing_decisions WHERE payment_id=$1
            "#,
        )
        .bind(payment_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| RoutingDecisionRecord {
            payment_id: r.get("payment_id"),
            selected_gateway: r.get("selected_gateway"),
            selected_score: r.get("selected_score"),
            runner_up_gateway: r.get("runner_up_gateway"),
            runner_up_score: r.get("runner_up_score"),
            strategy: r.get("strategy"),
            reason_summary: r.get("reason_summary"),
            score_breakdown_json: r.get("score_breakdown_json"),
            ranked_gateways_json: r.get("ranked_gateways_json"),
            created_at: r.get("created_at"),
        }))
    }
}

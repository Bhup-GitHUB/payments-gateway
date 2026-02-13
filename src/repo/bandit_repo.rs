use crate::bandit::thompson;
use anyhow::Result;
use sqlx::{PgPool, Row};

#[derive(Clone)]
pub struct BanditRepo {
    pub pool: PgPool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BanditStateRow {
    pub segment: String,
    pub gateway_id: String,
    pub alpha: f64,
    pub beta: f64,
}

impl BanditRepo {
    pub async fn is_enabled(&self, segment: &str) -> Result<bool> {
        let row = sqlx::query("SELECT enabled FROM bandit_policy WHERE segment=$1")
            .bind(segment)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| r.get::<bool, _>("enabled")).unwrap_or(false))
    }

    pub async fn set_enabled(&self, segment: &str, enabled: bool) -> Result<()> {
        sqlx::query(
            "INSERT INTO bandit_policy (segment, enabled, updated_at) VALUES ($1,$2,now()) ON CONFLICT (segment) DO UPDATE SET enabled=$2, updated_at=now()",
        )
        .bind(segment)
        .bind(enabled)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn ensure_gateway_state(&self, segment: &str, gateway_id: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO bandit_state (segment, gateway_id, alpha, beta, updated_at) VALUES ($1,$2,1.0,1.0,now()) ON CONFLICT (segment, gateway_id) DO NOTHING",
        )
        .bind(segment)
        .bind(gateway_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn sample_scores(&self, segment: &str, gateways: &[String]) -> Result<Vec<(String, f64)>> {
        let mut out = Vec::new();
        for gateway_id in gateways {
            self.ensure_gateway_state(segment, gateway_id).await?;
            let row = sqlx::query("SELECT alpha, beta FROM bandit_state WHERE segment=$1 AND gateway_id=$2")
                .bind(segment)
                .bind(gateway_id)
                .fetch_one(&self.pool)
                .await?;
            let alpha: f64 = row.get("alpha");
            let beta: f64 = row.get("beta");
            out.push((gateway_id.clone(), thompson::sample(alpha, beta)));
        }

        out.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(out)
    }

    pub async fn update_outcome(&self, segment: &str, gateway_id: &str, success: bool) -> Result<()> {
        self.ensure_gateway_state(segment, gateway_id).await?;
        if success {
            sqlx::query("UPDATE bandit_state SET alpha = alpha + 1, updated_at=now() WHERE segment=$1 AND gateway_id=$2")
                .bind(segment)
                .bind(gateway_id)
                .execute(&self.pool)
                .await?;
        } else {
            sqlx::query("UPDATE bandit_state SET beta = beta + 1, updated_at=now() WHERE segment=$1 AND gateway_id=$2")
                .bind(segment)
                .bind(gateway_id)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    pub async fn list_state(&self) -> Result<Vec<BanditStateRow>> {
        let rows = sqlx::query("SELECT segment, gateway_id, alpha, beta FROM bandit_state ORDER BY segment, gateway_id")
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(|row| BanditStateRow {
                segment: row.get("segment"),
                gateway_id: row.get("gateway_id"),
                alpha: row.get("alpha"),
                beta: row.get("beta"),
            })
            .collect())
    }
}

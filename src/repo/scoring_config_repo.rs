use anyhow::Result;
use chrono::{Datelike, Timelike};
use sqlx::{PgPool, Row};
use std::collections::HashMap;

#[derive(Clone)]
pub struct ScoringConfigRepo {
    pub pool: PgPool,
}

#[derive(Debug, Clone)]
pub struct ScoringWeights {
    pub success_rate_weight: f64,
    pub latency_weight: f64,
    pub method_affinity_weight: f64,
    pub bank_affinity_weight: f64,
    pub amount_fit_weight: f64,
    pub time_weight: f64,
}

impl ScoringConfigRepo {
    pub async fn load_weights(&self) -> Result<ScoringWeights> {
        let row = sqlx::query(
            "SELECT success_rate_weight, latency_weight, method_affinity_weight, bank_affinity_weight, amount_fit_weight, time_weight FROM scoring_config WHERE config_id='default'",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(ScoringWeights {
            success_rate_weight: row.get("success_rate_weight"),
            latency_weight: row.get("latency_weight"),
            method_affinity_weight: row.get("method_affinity_weight"),
            bank_affinity_weight: row.get("bank_affinity_weight"),
            amount_fit_weight: row.get("amount_fit_weight"),
            time_weight: row.get("time_weight"),
        })
    }

    pub async fn method_affinity(&self, gateway_id: &str, method: &str) -> Result<f64> {
        let row = sqlx::query(
            "SELECT score FROM gateway_method_affinity WHERE gateway_id=$1 AND payment_method=$2",
        )
        .bind(gateway_id)
        .bind(method)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.get("score")).unwrap_or(0.7))
    }

    pub async fn amount_fit(&self, gateway_id: &str, amount_bucket: &str) -> Result<f64> {
        let row = sqlx::query(
            "SELECT score FROM gateway_amount_fit WHERE gateway_id=$1 AND amount_bucket=$2",
        )
        .bind(gateway_id)
        .bind(amount_bucket)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.get("score")).unwrap_or(0.7))
    }

    pub async fn time_multiplier(&self, gateway_id: &str, now: chrono::DateTime<chrono::Utc>) -> Result<f64> {
        let hour = now.hour() as i32;
        let day = now.day() as i32;
        let row = sqlx::query(
            "SELECT multiplier FROM gateway_time_penalty WHERE gateway_id=$1 AND hour_of_day=$2 AND (day_of_month=$3 OR day_of_month IS NULL) ORDER BY day_of_month DESC NULLS LAST LIMIT 1",
        )
        .bind(gateway_id)
        .bind(hour)
        .bind(day)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.get("multiplier")).unwrap_or(1.0))
    }

    pub async fn resolve_bank_from_bin(&self, card_number: &str) -> Result<Option<String>> {
        if card_number.len() < 6 {
            return Ok(None);
        }
        let prefix = &card_number[..6];
        let row = sqlx::query("SELECT bank_code FROM bin_bank_map WHERE bin_prefix=$1")
            .bind(prefix)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| r.get::<String, _>("bank_code")))
    }

    pub async fn method_affinity_map(&self, method: &str) -> Result<HashMap<String, f64>> {
        let rows = sqlx::query("SELECT gateway_id, score FROM gateway_method_affinity WHERE payment_method=$1")
            .bind(method)
            .fetch_all(&self.pool)
            .await?;
        let mut out = HashMap::new();
        for row in rows {
            out.insert(row.get::<String, _>("gateway_id"), row.get::<f64, _>("score"));
        }
        Ok(out)
    }
}

use anyhow::Result;
use sqlx::{PgPool, Row};

#[derive(Clone)]
pub struct WebhookRepo {
    pub pool: PgPool,
}

#[derive(Debug, Clone)]
pub struct WebhookSubscription {
    pub event_type: String,
    pub target_url: String,
    pub secret: Option<String>,
}

impl WebhookRepo {
    pub async fn list_enabled_for_event(&self, event_type: &str) -> Result<Vec<WebhookSubscription>> {
        let rows = sqlx::query(
            "SELECT event_type, target_url, secret FROM webhook_subscriptions WHERE is_enabled=true AND event_type=$1",
        )
        .bind(event_type)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| WebhookSubscription {
                event_type: row.get("event_type"),
                target_url: row.get("target_url"),
                secret: row.get("secret"),
            })
            .collect())
    }
}

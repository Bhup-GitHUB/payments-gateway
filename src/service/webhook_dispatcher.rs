use crate::repo::webhook_repo::WebhookRepo;
use anyhow::Result;

#[derive(Clone)]
pub struct WebhookDispatcher {
    pub webhook_repo: WebhookRepo,
    pub client: reqwest::Client,
}

impl WebhookDispatcher {
    pub async fn emit(&self, event_type: &str, payload: serde_json::Value) -> Result<()> {
        let hooks = self.webhook_repo.list_enabled_for_event(event_type).await?;
        for hook in hooks {
            let mut req = self
                .client
                .post(&hook.target_url)
                .header("Content-Type", "application/json")
                .header("X-Event-Type", &hook.event_type)
                .json(&payload);
            if let Some(secret) = hook.secret {
                req = req.header("X-Webhook-Secret", secret);
            }
            let _ = req.send().await;
        }

        Ok(())
    }
}

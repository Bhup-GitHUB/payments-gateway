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
            let mut attempt = 0_u8;
            let mut backoff_ms = 150_u64;
            loop {
                let mut req = self
                    .client
                    .post(&hook.target_url)
                    .header("Content-Type", "application/json")
                    .header("X-Event-Type", &hook.event_type)
                    .json(&payload);
                if let Some(secret) = &hook.secret {
                    req = req.header("X-Webhook-Secret", secret);
                }

                let retry = match req.send().await {
                    Ok(resp) => resp.status().is_server_error(),
                    Err(_) => true,
                };

                if !retry || attempt >= 2 {
                    break;
                }

                tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                backoff_ms *= 2;
                attempt += 1;
            }
        }

        Ok(())
    }
}

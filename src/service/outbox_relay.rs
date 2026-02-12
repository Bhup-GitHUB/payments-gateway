use crate::repo::outbox_repo::OutboxRepo;
use anyhow::Result;
use chrono::{Duration, Utc};

#[derive(Clone)]
pub struct OutboxRelay {
    pub outbox_repo: OutboxRepo,
    pub redis_client: redis::Client,
    pub stream_key: String,
}

impl OutboxRelay {
    pub async fn run(self) {
        loop {
            if let Err(err) = self.tick().await {
                tracing::error!("outbox relay error: {}", err);
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
    }

    async fn tick(&self) -> Result<()> {
        let batch = self.outbox_repo.lock_pending(100).await?;
        if batch.is_empty() {
            return Ok(());
        }

        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;
        for item in batch {
            let payload = serde_json::to_string(&item.payload_json)?;
            let add_res: redis::RedisResult<String> = redis::cmd("XADD")
                .arg(&self.stream_key)
                .arg("MAXLEN")
                .arg("~")
                .arg(1_000_000)
                .arg("*")
                .arg("event")
                .arg(payload)
                .query_async(&mut conn)
                .await;

            match add_res {
                Ok(_) => {
                    self.outbox_repo.mark_published(item.id).await?;
                }
                Err(e) => {
                    let attempts = item.attempts + 1;
                    let backoff = i64::min(300, 2_i64.pow((attempts.min(8)) as u32));
                    let next_attempt_at = Utc::now() + Duration::seconds(backoff);
                    self.outbox_repo.mark_retry(item.id, attempts, next_attempt_at).await?;
                    tracing::warn!("xadd failed for outbox id {}: {}", item.id, e);
                }
            }
        }

        Ok(())
    }
}

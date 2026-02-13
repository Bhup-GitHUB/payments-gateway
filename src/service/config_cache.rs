use crate::repo::scoring_config_repo::{ScoringConfigRepo, ScoringWeights};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct ConfigCache {
    pub scoring_repo: ScoringConfigRepo,
    inner: Arc<RwLock<Option<(std::time::Instant, ScoringWeights)>>>,
    ttl: std::time::Duration,
}

impl ConfigCache {
    pub fn new(scoring_repo: ScoringConfigRepo, ttl: std::time::Duration) -> Self {
        Self {
            scoring_repo,
            inner: Arc::new(RwLock::new(None)),
            ttl,
        }
    }

    pub async fn scoring_weights(&self) -> Result<ScoringWeights> {
        {
            let read = self.inner.read().await;
            if let Some((loaded_at, weights)) = &*read {
                if loaded_at.elapsed() <= self.ttl {
                    return Ok(weights.clone());
                }
            }
        }

        let weights = self.scoring_repo.load_weights().await?;
        let mut write = self.inner.write().await;
        *write = Some((std::time::Instant::now(), weights.clone()));
        Ok(weights)
    }
}

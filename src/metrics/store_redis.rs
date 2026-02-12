use crate::metrics::aggregator::AggregatedMetric;
use crate::metrics::aggregator::MetricKey;
use anyhow::Result;
use redis::AsyncCommands;

#[derive(Clone)]
pub struct MetricsHotStore {
    pub client: redis::Client,
}

impl MetricsHotStore {
    pub fn new(redis_url: &str) -> Result<Self> {
        Ok(Self {
            client: redis::Client::open(redis_url)?,
        })
    }

    pub fn metric_key(key: &MetricKey, window: i64) -> String {
        format!(
            "metrics:{}:{}:{}:{}m",
            key.gateway.to_lowercase(),
            key.method.to_lowercase(),
            key.bank.to_lowercase(),
            window
        )
    }

    pub fn index_key(gateway: &str, window: i64) -> String {
        format!("metrics:index:{}:{}m", gateway.to_lowercase(), window)
    }

    pub async fn write_metric(&self, key: &MetricKey, window: i64, metric: &AggregatedMetric) -> Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let redis_key = Self::metric_key(key, window);
        let index_key = Self::index_key(&key.gateway, window);
        let member = format!("{}:{}", key.method.to_lowercase(), key.bank.to_lowercase());
        let payload = serde_json::to_string(metric)?;
        let ttl = (window * 60 + 120) as u64;

        let _: () = conn.set_ex(&redis_key, payload, ttl).await?;
        let _: usize = conn.sadd(index_key.clone(), member).await?;
        let _: bool = conn.expire(index_key, ttl as i64).await?;
        Ok(())
    }

    pub async fn read_gateway_metrics(
        &self,
        gateway: &str,
        window: i64,
        filter_method: Option<&str>,
        filter_bank: Option<&str>,
    ) -> Result<Vec<(String, String, AggregatedMetric)>> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let index_key = Self::index_key(gateway, window);
        let members: Vec<String> = conn.smembers(index_key).await.unwrap_or_default();

        let mut out = Vec::new();
        for member in members {
            let parts: Vec<&str> = member.split(':').collect();
            if parts.len() != 2 {
                continue;
            }
            let method = parts[0].to_string();
            let bank = parts[1].to_string();
            if filter_method.is_some_and(|m| m.to_lowercase() != method) {
                continue;
            }
            if filter_bank.is_some_and(|b| b.to_lowercase() != bank) {
                continue;
            }
            let redis_key = format!("metrics:{}:{}:{}:{}m", gateway.to_lowercase(), method, bank, window);
            let payload: Option<String> = conn.get(redis_key).await.ok();
            if let Some(p) = payload {
                if let Ok(metric) = serde_json::from_str::<AggregatedMetric>(&p) {
                    out.push((method, bank, metric));
                }
            }
        }

        Ok(out)
    }
}

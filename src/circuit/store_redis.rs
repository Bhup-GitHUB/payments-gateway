use crate::circuit::state::CircuitSnapshot;
use anyhow::Result;
use chrono::Timelike;
use redis::AsyncCommands;

#[derive(Clone)]
pub struct CircuitStoreRedis {
    pub client: redis::Client,
}

impl CircuitStoreRedis {
    pub fn new(client: redis::Client) -> Self {
        Self { client }
    }

    fn state_key(gateway_id: &str, method: &str) -> String {
        format!("circuit:state:{}:{}", gateway_id.to_lowercase(), method.to_lowercase())
    }

    fn minute_stats_key(gateway_id: &str, method: &str, minute_epoch: i64) -> String {
        format!(
            "circuit:stats:{}:{}:{}",
            gateway_id.to_lowercase(),
            method.to_lowercase(),
            minute_epoch
        )
    }

    fn override_key(gateway_id: &str, method: &str) -> String {
        format!("circuit:manual_override:{}:{}", gateway_id.to_lowercase(), method.to_lowercase())
    }

    pub async fn get_snapshot(&self, gateway_id: &str, method: &str) -> Result<CircuitSnapshot> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = Self::state_key(gateway_id, method);
        let payload: Option<String> = conn.get(key).await?;
        if let Some(payload) = payload {
            let parsed = serde_json::from_str::<CircuitSnapshot>(&payload)?;
            return Ok(parsed);
        }
        Ok(CircuitSnapshot::new(gateway_id, method))
    }

    pub async fn save_snapshot(&self, snapshot: &CircuitSnapshot) -> Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = Self::state_key(&snapshot.gateway_id, &snapshot.payment_method);
        let payload = serde_json::to_string(snapshot)?;
        let _: () = conn.set(key, payload).await?;
        Ok(())
    }

    pub async fn write_result(
        &self,
        gateway_id: &str,
        method: &str,
        status: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        let minute = now.timestamp() - (now.second() as i64);
        let key = Self::minute_stats_key(gateway_id, method, minute);
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let _: f64 = redis::cmd("HINCRBYFLOAT")
            .arg(&key)
            .arg("total")
            .arg(1.0)
            .query_async(&mut conn)
            .await?;

        if status == "SUCCESS" {
            let _: f64 = redis::cmd("HINCRBYFLOAT")
                .arg(&key)
                .arg("success")
                .arg(1.0)
                .query_async(&mut conn)
                .await?;
        } else if status == "TIMEOUT" {
            let _: f64 = redis::cmd("HINCRBYFLOAT")
                .arg(&key)
                .arg("failed")
                .arg(1.0)
                .query_async(&mut conn)
                .await?;
            let _: f64 = redis::cmd("HINCRBYFLOAT")
                .arg(&key)
                .arg("timeout")
                .arg(1.0)
                .query_async(&mut conn)
                .await?;
        } else {
            let _: f64 = redis::cmd("HINCRBYFLOAT")
                .arg(&key)
                .arg("failed")
                .arg(1.0)
                .query_async(&mut conn)
                .await?;
        }

        let _: bool = conn.expire(&key, 600).await?;
        Ok(())
    }

    pub async fn aggregate_window(
        &self,
        gateway_id: &str,
        method: &str,
        minutes: i64,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<(f64, f64)> {
        let mut total = 0.0;
        let mut failed = 0.0;
        let mut timeout = 0.0;

        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let current = now.timestamp() - (now.second() as i64);
        for i in 0..minutes {
            let key = Self::minute_stats_key(gateway_id, method, current - (i * 60));
            let values: std::collections::HashMap<String, String> = conn.hgetall(key).await.unwrap_or_default();
            total += values.get("total").and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
            failed += values.get("failed").and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
            timeout += values.get("timeout").and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
        }

        if total <= 0.0 {
            return Ok((0.0, 0.0));
        }

        Ok((failed / total, timeout / total))
    }

    pub async fn get_override(&self, gateway_id: &str, method: &str) -> Result<Option<String>> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = Self::override_key(gateway_id, method);
        let val: Option<String> = conn.get(key).await?;
        Ok(val)
    }

    pub async fn set_override(&self, gateway_id: &str, method: &str, value: &str) -> Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = Self::override_key(gateway_id, method);
        let _: () = conn.set(key, value).await?;
        Ok(())
    }

    pub async fn clear_override(&self, gateway_id: &str, method: &str) -> Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = Self::override_key(gateway_id, method);
        let _: usize = conn.del(key).await?;
        Ok(())
    }
}

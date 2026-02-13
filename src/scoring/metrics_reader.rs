use crate::metrics::store_redis::MetricsHotStore;
use anyhow::Result;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct GatewayMetricInput {
    pub success_rate: f64,
    pub p95_latency_ms: i32,
}

pub async fn read_metric_for_gateway(
    store: &MetricsHotStore,
    gateway: &str,
    method: &str,
    bank: &str,
) -> Result<GatewayMetricInput> {
    if let Some(metric) = store.read_single_metric(gateway, method, bank, 5).await? {
        return Ok(GatewayMetricInput {
            success_rate: metric.success_rate,
            p95_latency_ms: metric.p95_latency_ms,
        });
    }

    Ok(GatewayMetricInput {
        success_rate: 0.5,
        p95_latency_ms: 1500,
    })
}

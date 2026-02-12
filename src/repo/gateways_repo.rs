use crate::gateways::GatewayConfig;
use sqlx::{PgPool, Row};

#[derive(Clone)]
pub struct GatewaysRepo {
    pub pool: PgPool,
}

impl GatewaysRepo {
    pub async fn list_all(&self) -> anyhow::Result<Vec<GatewayConfig>> {
        let rows = sqlx::query(
            "SELECT gateway_id, gateway_name, adapter_type, is_enabled, priority, supported_methods, timeout_ms, mock_behavior FROM gateways_config ORDER BY priority ASC, gateway_name ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| GatewayConfig {
                gateway_id: r.get("gateway_id"),
                gateway_name: r.get("gateway_name"),
                adapter_type: r.get("adapter_type"),
                is_enabled: r.get("is_enabled"),
                priority: r.get("priority"),
                supported_methods: r.get("supported_methods"),
                timeout_ms: r.get("timeout_ms"),
                mock_behavior: r.get("mock_behavior"),
            })
            .collect())
    }

    pub async fn list_enabled_by_method(&self, method: &str) -> anyhow::Result<Vec<GatewayConfig>> {
        let rows = sqlx::query(
            "SELECT gateway_id, gateway_name, adapter_type, is_enabled, priority, supported_methods, timeout_ms, mock_behavior FROM gateways_config WHERE is_enabled = true AND $1 = ANY(supported_methods) ORDER BY priority ASC, gateway_name ASC",
        )
        .bind(method)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| GatewayConfig {
                gateway_id: r.get("gateway_id"),
                gateway_name: r.get("gateway_name"),
                adapter_type: r.get("adapter_type"),
                is_enabled: r.get("is_enabled"),
                priority: r.get("priority"),
                supported_methods: r.get("supported_methods"),
                timeout_ms: r.get("timeout_ms"),
                mock_behavior: r.get("mock_behavior"),
            })
            .collect())
    }

    pub async fn update_gateway(
        &self,
        gateway_id: &str,
        is_enabled: bool,
        priority: i32,
        supported_methods: Vec<String>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE gateways_config SET is_enabled = $2, priority = $3, supported_methods = $4, updated_at = now() WHERE gateway_id = $1",
        )
        .bind(gateway_id)
        .bind(is_enabled)
        .bind(priority)
        .bind(supported_methods)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

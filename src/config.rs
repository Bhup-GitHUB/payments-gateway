#[derive(Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub bind_addr: String,
    pub redis_url: String,
    pub stream_key: String,
    pub stream_group: String,
    pub internal_api_key: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/payments_gateway".to_string()),
            bind_addr: std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string()),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string()),
            stream_key: std::env::var("METRICS_STREAM_KEY")
                .unwrap_or_else(|_| "payments:events:v1".to_string()),
            stream_group: std::env::var("METRICS_STREAM_GROUP")
                .unwrap_or_else(|_| "metrics-agg-v1".to_string()),
            internal_api_key: std::env::var("INTERNAL_API_KEY")
                .unwrap_or_else(|_| "dev-internal-key".to_string()),
        }
    }
}

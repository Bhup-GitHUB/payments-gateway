use anyhow::Result;
use payments_gateway::config::AppConfig;
use payments_gateway::repo::payment_verification_repo::PaymentVerificationRepo;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cfg = AppConfig::from_env();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&cfg.database_url)
        .await?;

    let repo = PaymentVerificationRepo { pool };

    loop {
        let due = repo.due_items(100).await?;
        for row in due {
            let next = chrono::Utc::now() + chrono::Duration::minutes(2);
            repo.mark(
                row.payment_id,
                if row.attempts >= 2 { "EXHAUSTED" } else { "PENDING" },
                row.attempts + 1,
                serde_json::json!({"note": "status check placeholder"}),
                Some(next),
            )
            .await?;
        }

        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
}

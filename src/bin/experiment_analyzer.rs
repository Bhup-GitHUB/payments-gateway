use anyhow::Result;
use payments_gateway::experiments::analyzer::analyze;
use payments_gateway::repo::experiments_repo::ExperimentsRepo;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/payments_gateway".to_string());
    let pool = PgPoolOptions::new().max_connections(5).connect(&database_url).await?;
    let repo = ExperimentsRepo { pool };

    loop {
        let experiments = repo.list().await?;
        for exp in experiments.into_iter().filter(|e| e.status == "RUNNING") {
            let results = repo.results(exp.experiment_id).await?;
            let out = analyze(&results, 100);
            tracing::info!(
                "experiment={} winner={:?} p_value={} recommendation={}",
                exp.experiment_id,
                out.winner,
                out.p_value,
                out.recommendation
            );
        }

        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
    }
}

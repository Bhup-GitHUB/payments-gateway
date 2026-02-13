use anyhow::Result;
use payments_gateway::experiments::analyzer::{analyze, evaluate_guardrails, GuardrailConfig};
use payments_gateway::repo::experiments_repo::ExperimentsRepo;
use payments_gateway::repo::webhook_repo::WebhookRepo;
use payments_gateway::service::webhook_dispatcher::WebhookDispatcher;
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
    let repo = ExperimentsRepo { pool: pool.clone() };
    let webhook_dispatcher = WebhookDispatcher {
        webhook_repo: WebhookRepo { pool },
        client: reqwest::Client::new(),
    };
    let guardrails = GuardrailConfig {
        min_samples: std::env::var("EXPERIMENT_GUARDRAIL_MIN_SAMPLES")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(100),
        max_success_rate_drop: std::env::var("EXPERIMENT_GUARDRAIL_MAX_SUCCESS_DROP")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.05),
        max_latency_multiplier: std::env::var("EXPERIMENT_GUARDRAIL_MAX_LATENCY_MULTIPLIER")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(1.5),
    };

    loop {
        let experiments = repo.list().await?;
        for exp in experiments.into_iter().filter(|e| e.status == "RUNNING") {
            let results = repo.results(exp.experiment_id).await?;
            let out = analyze(&results, 100);
            let guardrail = evaluate_guardrails(&results, &guardrails);
            tracing::info!(
                "experiment={} winner={:?} p_value={} recommendation={}",
                exp.experiment_id,
                out.winner,
                out.p_value,
                out.recommendation
            );
            if guardrail.should_pause {
                repo.stop(exp.experiment_id).await?;
                let _ = webhook_dispatcher
                    .emit(
                        "experiment.guardrail_violation",
                        serde_json::json!({
                            "experiment_id": exp.experiment_id,
                            "reason": guardrail.reason,
                            "control_success_rate": guardrail.control_success_rate,
                            "treatment_success_rate": guardrail.treatment_success_rate,
                            "control_p95_latency_ms": guardrail.control_p95_latency_ms,
                            "treatment_p95_latency_ms": guardrail.treatment_p95_latency_ms
                        }),
                    )
                    .await;
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
    }
}

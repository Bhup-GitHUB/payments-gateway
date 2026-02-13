use axum::routing::{get, patch, post};
use axum::Router;
use payments_gateway::config::AppConfig;
use payments_gateway::circuit::store_redis::CircuitStoreRedis;
use payments_gateway::gateways::razorpay::RazorpayGateway;
use payments_gateway::metrics::store_redis::MetricsHotStore;
use payments_gateway::repo::circuit_breaker_config_repo::CircuitBreakerConfigRepo;
use payments_gateway::repo::gateways_repo::GatewaysRepo;
use payments_gateway::repo::outbox_repo::OutboxRepo;
use payments_gateway::repo::payments_repo::PaymentsRepo;
use payments_gateway::repo::routing_decisions_repo::RoutingDecisionsRepo;
use payments_gateway::repo::scoring_config_repo::ScoringConfigRepo;
use payments_gateway::service::outbox_relay::OutboxRelay;
use payments_gateway::service::payment_service::PaymentService;
use payments_gateway::AppState;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cfg = AppConfig::from_env();

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&cfg.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let redis_client = redis::Client::open(cfg.redis_url.clone())?;
    let metrics_hot_store = MetricsHotStore::new(&cfg.redis_url)?;

    let gateways_repo = GatewaysRepo { pool: pool.clone() };
    let payments_repo = PaymentsRepo { pool: pool.clone() };
    let outbox_repo = OutboxRepo { pool: pool.clone() };
    let scoring_config_repo = ScoringConfigRepo { pool: pool.clone() };
    let routing_decisions_repo = RoutingDecisionsRepo { pool: pool.clone() };
    let circuit_breaker_config_repo = CircuitBreakerConfigRepo { pool: pool.clone() };
    let razorpay = Arc::new(RazorpayGateway {
        base_url: std::env::var("RAZORPAY_BASE_URL")
            .unwrap_or_else(|_| "https://api.razorpay.com".to_string()),
        key_id: std::env::var("RAZORPAY_KEY_ID").unwrap_or_default(),
        key_secret: std::env::var("RAZORPAY_KEY_SECRET").unwrap_or_default(),
        timeout_ms: std::env::var("GATEWAY_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(2500),
        client: reqwest::Client::new(),
    });

    let payment_service = PaymentService {
        pool: pool.clone(),
        payments_repo,
        outbox_repo: outbox_repo.clone(),
        gateways_repo: gateways_repo.clone(),
        scoring_config_repo,
        routing_decisions_repo: routing_decisions_repo.clone(),
        circuit_breaker_config_repo: circuit_breaker_config_repo.clone(),
        metrics_hot_store: metrics_hot_store.clone(),
        circuit_store: CircuitStoreRedis::new(redis::Client::open(cfg.redis_url.clone())?),
        razorpay,
    };

    let relay = OutboxRelay {
        outbox_repo,
        redis_client,
        stream_key: cfg.stream_key.clone(),
    };
    tokio::spawn(relay.run());

    let state = AppState {
        payment_service,
        gateways_repo,
        metrics_hot_store,
        routing_decisions_repo,
        circuit_breaker_config_repo,
        redis_client: redis::Client::open(cfg.redis_url.clone())?,
    };

    let app = Router::new()
        .route("/health", get(payments_gateway::http::handlers::payments::health))
        .route("/payments", post(payments_gateway::http::handlers::payments::create_payment))
        .route(
            "/payments/:payment_id/routing-decision",
            get(payments_gateway::http::handlers::routing_decisions::get_routing_decision),
        )
        .route("/gateways", get(payments_gateway::http::handlers::gateways::list_gateways))
        .route(
            "/gateways/:gateway_id",
            patch(payments_gateway::http::handlers::gateways::update_gateway),
        )
        .route(
            "/metrics/gateways/:gateway_name",
            get(payments_gateway::http::handlers::metrics::get_gateway_metrics),
        )
        .route(
            "/scoring/debug",
            get(payments_gateway::http::handlers::scoring_debug::scoring_debug),
        )
        .route(
            "/circuit-breaker/status",
            get(payments_gateway::http::handlers::circuit_breaker::status),
        )
        .route(
            "/circuit-breaker/force-open/:gateway/:method",
            post(payments_gateway::http::handlers::circuit_breaker::force_open),
        )
        .route(
            "/circuit-breaker/force-close/:gateway/:method",
            post(payments_gateway::http::handlers::circuit_breaker::force_close),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&cfg.bind_addr).await?;
    tracing::info!("listening on {}", cfg.bind_addr);
    axum::serve(listener, app).await?;
    Ok(())
}

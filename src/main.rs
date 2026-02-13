use axum::middleware::from_fn_with_state;
use axum::routing::{get, patch, post, put};
use axum::Router;
use payments_gateway::config::AppConfig;
use payments_gateway::circuit::store_redis::CircuitStoreRedis;
use payments_gateway::gateways::razorpay::RazorpayGateway;
use payments_gateway::metrics::store_redis::MetricsHotStore;
use payments_gateway::repo::circuit_breaker_config_repo::CircuitBreakerConfigRepo;
use payments_gateway::repo::bandit_repo::BanditRepo;
use payments_gateway::repo::error_classification_repo::ErrorClassificationRepo;
use payments_gateway::repo::experiments_repo::ExperimentsRepo;
use payments_gateway::repo::gateways_repo::GatewaysRepo;
use payments_gateway::repo::outbox_repo::OutboxRepo;
use payments_gateway::repo::payment_attempts_repo::PaymentAttemptsRepo;
use payments_gateway::repo::payment_verification_repo::PaymentVerificationRepo;
use payments_gateway::repo::payments_repo::PaymentsRepo;
use payments_gateway::repo::retry_policy_repo::RetryPolicyRepo;
use payments_gateway::repo::routing_decisions_repo::RoutingDecisionsRepo;
use payments_gateway::repo::scoring_config_repo::ScoringConfigRepo;
use payments_gateway::repo::webhook_repo::WebhookRepo;
use payments_gateway::service::config_cache::ConfigCache;
use payments_gateway::service::outbox_relay::OutboxRelay;
use payments_gateway::service::payment_service::PaymentService;
use payments_gateway::service::webhook_dispatcher::WebhookDispatcher;
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
    let payment_attempts_repo = PaymentAttemptsRepo { pool: pool.clone() };
    let retry_policy_repo = RetryPolicyRepo { pool: pool.clone() };
    let error_classification_repo = ErrorClassificationRepo { pool: pool.clone() };
    let payment_verification_repo = PaymentVerificationRepo { pool: pool.clone() };
    let experiments_repo = ExperimentsRepo { pool: pool.clone() };
    let bandit_repo = BanditRepo { pool: pool.clone() };
    let webhook_repo = WebhookRepo { pool: pool.clone() };
    let webhook_dispatcher = WebhookDispatcher {
        webhook_repo: webhook_repo.clone(),
        client: reqwest::Client::new(),
    };
    let config_cache = ConfigCache::new(scoring_config_repo.clone(), std::time::Duration::from_secs(300));
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
        experiments_repo: experiments_repo.clone(),
        bandit_repo: bandit_repo.clone(),
        scoring_config_repo,
        routing_decisions_repo: routing_decisions_repo.clone(),
        circuit_breaker_config_repo: circuit_breaker_config_repo.clone(),
        metrics_hot_store: metrics_hot_store.clone(),
        circuit_store: CircuitStoreRedis::new(redis::Client::open(cfg.redis_url.clone())?),
        payment_attempts_repo: payment_attempts_repo.clone(),
        retry_policy_repo: retry_policy_repo.clone(),
        error_classification_repo: error_classification_repo.clone(),
        payment_verification_repo: payment_verification_repo.clone(),
        webhook_dispatcher: webhook_dispatcher.clone(),
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
        experiments_repo: experiments_repo.clone(),
        routing_decisions_repo,
        circuit_breaker_config_repo,
        payment_attempts_repo,
        retry_policy_repo,
        payment_verification_repo,
        bandit_repo: bandit_repo.clone(),
        redis_client: redis::Client::open(cfg.redis_url.clone())?,
        webhook_dispatcher: webhook_dispatcher.clone(),
        config_cache,
        stream_key: cfg.stream_key.clone(),
    };

    let admin_key = cfg.internal_api_key.clone();
    let admin_routes = Router::new()
        .route(
            "/retry-policy/:merchant_id",
            put(payments_gateway::http::handlers::retry_policy::upsert_retry_policy),
        )
        .route(
            "/circuit-breaker/force-open/:gateway/:method",
            post(payments_gateway::http::handlers::circuit_breaker::force_open),
        )
        .route(
            "/circuit-breaker/force-close/:gateway/:method",
            post(payments_gateway::http::handlers::circuit_breaker::force_close),
        )
        .route(
            "/experiments",
            post(payments_gateway::http::handlers::experiments::create_experiment),
        )
        .route(
            "/experiments/:id/stop",
            post(payments_gateway::http::handlers::experiments::stop_experiment),
        )
        .route(
            "/bandit/policy/:segment/enable",
            post(payments_gateway::http::handlers::bandit::enable_segment),
        )
        .route(
            "/bandit/policy/:segment/disable",
            post(payments_gateway::http::handlers::bandit::disable_segment),
        )
        .layer(from_fn_with_state(
            admin_key,
            payments_gateway::http::middleware::admin_auth::require_internal_api_key,
        ));

    let app = Router::new()
        .route("/health", get(payments_gateway::http::handlers::payments::health))
        .route("/payments", post(payments_gateway::http::handlers::payments::create_payment))
        .route(
            "/payments/:payment_id/routing-decision",
            get(payments_gateway::http::handlers::routing_decisions::get_routing_decision),
        )
        .route(
            "/payments/:payment_id/attempts",
            get(payments_gateway::http::handlers::payment_attempts::list_attempts),
        )
        .route(
            "/payments/:payment_id/status-verification",
            get(payments_gateway::http::handlers::payment_attempts::get_status_verification),
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
            "/retry-policy/:merchant_id",
            get(payments_gateway::http::handlers::retry_policy::get_retry_policy),
        )
        .route("/experiments", get(payments_gateway::http::handlers::experiments::list_experiments))
        .route(
            "/experiments/:id/results",
            get(payments_gateway::http::handlers::experiments::get_results),
        )
        .route(
            "/experiments/:id/winner",
            get(payments_gateway::http::handlers::experiment_winner::get_experiment_winner),
        )
        .route("/bandit/state", get(payments_gateway::http::handlers::bandit::get_state))
        .route("/ops/readiness", get(payments_gateway::http::handlers::ops::readiness))
        .route("/ops/liveness", get(payments_gateway::http::handlers::ops::liveness))
        .merge(admin_routes)
        .layer(from_fn_with_state(
            payments_gateway::http::middleware::rate_limit::RateLimitState {
                redis_client: redis::Client::open(cfg.redis_url.clone())?,
                max_per_minute: 300,
            },
            payments_gateway::http::middleware::rate_limit::enforce,
        ))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&cfg.bind_addr).await?;
    tracing::info!("listening on {}", cfg.bind_addr);
    axum::serve(listener, app).await?;
    Ok(())
}

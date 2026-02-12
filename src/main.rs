mod domain {
    pub mod context;
    pub mod payment;
}
mod gateways;
mod http {
    pub mod handlers {
        pub mod gateways;
        pub mod payments;
    }
}
mod repo {
    pub mod gateways_repo;
    pub mod payments_repo;
}
mod router {
    pub mod round_robin;
}
mod service {
    pub mod payment_service;
}

use axum::routing::{get, patch, post};
use axum::Router;
use gateways::razorpay::RazorpayGateway;
use repo::gateways_repo::GatewaysRepo;
use repo::payments_repo::PaymentsRepo;
use service::payment_service::PaymentService;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct AppState {
    payment_service: PaymentService,
    gateways_repo: GatewaysRepo,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/payments_gateway".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let gateways_repo = GatewaysRepo { pool: pool.clone() };
    let payments_repo = PaymentsRepo { pool: pool.clone() };
    let router = Arc::new(router::round_robin::RoundRobinRouter::new());
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
        payments_repo,
        gateways_repo: gateways_repo.clone(),
        router,
        razorpay,
    };

    let state = AppState {
        payment_service,
        gateways_repo,
    };

    let app = Router::new()
        .route("/health", get(http::handlers::payments::health))
        .route("/payments", post(http::handlers::payments::create_payment))
        .route("/gateways", get(http::handlers::gateways::list_gateways))
        .route(
            "/gateways/:gateway_id",
            patch(http::handlers::gateways::update_gateway),
        )
        .with_state(state);

    let addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

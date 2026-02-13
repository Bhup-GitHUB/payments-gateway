pub mod config;
pub mod domain {
    pub mod context;
    pub mod payment;
    pub mod routing_decision;
}
pub mod gateways;
pub mod http {
    pub mod handlers {
        pub mod circuit_breaker;
        pub mod gateways;
        pub mod metrics;
        pub mod payments;
        pub mod routing_decisions;
        pub mod scoring_debug;
    }
}
pub mod metrics;
pub mod repo {
    pub mod circuit_breaker_config_repo;
    pub mod error_classification_repo;
    pub mod gateways_repo;
    pub mod outbox_repo;
    pub mod payment_attempts_repo;
    pub mod payments_repo;
    pub mod routing_decisions_repo;
    pub mod retry_policy_repo;
    pub mod scoring_config_repo;
}
pub mod router {
    pub mod round_robin;
}
pub mod scoring;
pub mod circuit;
pub mod service {
    pub mod outbox_relay;
    pub mod payment_service;
}

#[derive(Clone)]
pub struct AppState {
    pub payment_service: service::payment_service::PaymentService,
    pub gateways_repo: repo::gateways_repo::GatewaysRepo,
    pub metrics_hot_store: metrics::store_redis::MetricsHotStore,
    pub routing_decisions_repo: repo::routing_decisions_repo::RoutingDecisionsRepo,
    pub circuit_breaker_config_repo: repo::circuit_breaker_config_repo::CircuitBreakerConfigRepo,
    pub redis_client: redis::Client,
}

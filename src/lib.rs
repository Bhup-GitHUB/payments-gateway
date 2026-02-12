pub mod config;
pub mod domain {
    pub mod context;
    pub mod payment;
}
pub mod gateways;
pub mod http {
    pub mod handlers {
        pub mod gateways;
        pub mod metrics;
        pub mod payments;
    }
}
pub mod metrics;
pub mod repo {
    pub mod gateways_repo;
    pub mod outbox_repo;
    pub mod payments_repo;
}
pub mod router {
    pub mod round_robin;
}
pub mod service {
    pub mod outbox_relay;
    pub mod payment_service;
}

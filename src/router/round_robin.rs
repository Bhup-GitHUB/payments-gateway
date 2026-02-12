use crate::gateways::GatewayConfig;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct RoundRobinRouter {
    counter: AtomicUsize,
}

impl RoundRobinRouter {
    pub fn new() -> Self {
        Self {
            counter: AtomicUsize::new(0),
        }
    }

    pub fn select(&self, candidates: &[GatewayConfig]) -> Option<(GatewayConfig, String)> {
        if candidates.is_empty() {
            return None;
        }
        let idx = self.counter.fetch_add(1, Ordering::Relaxed) % candidates.len();
        let selected = candidates[idx].clone();
        let reason = format!("round_robin(index={},total={})", idx, candidates.len());
        Some((selected, reason))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_in_round_robin_order() {
        let router = RoundRobinRouter::new();
        let gateways = vec![
            GatewayConfig {
                gateway_id: "g1".to_string(),
                gateway_name: "g1".to_string(),
                adapter_type: "MOCK".to_string(),
                is_enabled: true,
                priority: 1,
                supported_methods: vec!["UPI".to_string()],
                timeout_ms: 1000,
                mock_behavior: None,
            },
            GatewayConfig {
                gateway_id: "g2".to_string(),
                gateway_name: "g2".to_string(),
                adapter_type: "MOCK".to_string(),
                is_enabled: true,
                priority: 2,
                supported_methods: vec!["UPI".to_string()],
                timeout_ms: 1000,
                mock_behavior: None,
            },
        ];

        let a = router.select(&gateways).unwrap().0.gateway_id;
        let b = router.select(&gateways).unwrap().0.gateway_id;
        let c = router.select(&gateways).unwrap().0.gateway_id;

        assert_eq!(a, "g1");
        assert_eq!(b, "g2");
        assert_eq!(c, "g1");
    }
}

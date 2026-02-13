use payments_gateway::gateways::GatewayConfig;
use payments_gateway::scoring::engine::rank_gateways;
use payments_gateway::scoring::types::{GatewayCandidate, ScoreInputs, ScoreWeights};

#[test]
fn scoring_prefers_better_gateway() {
    let weights = ScoreWeights {
        success_rate_weight: 0.35,
        latency_weight: 0.25,
        method_affinity_weight: 0.15,
        bank_affinity_weight: 0.12,
        amount_fit_weight: 0.08,
        time_weight: 0.05,
    };

    let top = GatewayCandidate {
        gateway: gateway("hdfc_mock"),
        inputs: ScoreInputs {
            success_rate: 0.95,
            p95_latency_ms: 700,
            method_affinity: 0.9,
            bank_affinity: 1.0,
            amount_fit: 1.0,
            time_multiplier: 1.0,
        },
    };

    let low = GatewayCandidate {
        gateway: gateway("axis_mock"),
        inputs: ScoreInputs {
            success_rate: 0.75,
            p95_latency_ms: 1800,
            method_affinity: 0.7,
            bank_affinity: 0.5,
            amount_fit: 0.7,
            time_multiplier: 0.9,
        },
    };

    let ranked = rank_gateways(&[low, top], &weights);
    assert_eq!(ranked.first().unwrap().gateway_id, "hdfc_mock");
}

fn gateway(id: &str) -> GatewayConfig {
    GatewayConfig {
        gateway_id: id.to_string(),
        gateway_name: id.to_string(),
        adapter_type: "MOCK".to_string(),
        is_enabled: true,
        priority: 1,
        supported_methods: vec!["UPI".to_string()],
        timeout_ms: 1000,
        mock_behavior: None,
    }
}

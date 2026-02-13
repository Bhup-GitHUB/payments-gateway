use crate::scoring::types::{GatewayCandidate, RankedGateway, ScoreBreakdown, ScoreWeights};

pub fn latency_score(p95_latency_ms: i32) -> f64 {
    1.0 / (1.0 + (p95_latency_ms as f64 / 1000.0))
}

pub fn clamp01(v: f64) -> f64 {
    if v < 0.0 {
        0.0
    } else if v > 1.0 {
        1.0
    } else {
        v
    }
}

pub fn score_gateway(candidate: &GatewayCandidate, weights: &ScoreWeights) -> RankedGateway {
    let success_rate_score = clamp01(candidate.inputs.success_rate);
    let latency_component = clamp01(latency_score(candidate.inputs.p95_latency_ms));
    let method_affinity = clamp01(candidate.inputs.method_affinity);
    let bank_affinity = clamp01(candidate.inputs.bank_affinity);
    let amount_fit = clamp01(candidate.inputs.amount_fit);
    let time_weight = clamp01(candidate.inputs.time_multiplier);

    let raw = (weights.success_rate_weight * success_rate_score)
        + (weights.latency_weight * latency_component)
        + (weights.method_affinity_weight * method_affinity)
        + (weights.bank_affinity_weight * bank_affinity)
        + (weights.amount_fit_weight * amount_fit)
        + (weights.time_weight * time_weight);

    let final_score = clamp01(raw);

    RankedGateway {
        gateway_id: candidate.gateway.gateway_id.clone(),
        score: final_score,
        breakdown: ScoreBreakdown {
            success_rate_score,
            latency_score: latency_component,
            method_affinity,
            bank_affinity,
            amount_fit,
            time_weight,
            final_score,
        },
    }
}

pub fn rank_gateways(candidates: &[GatewayCandidate], weights: &ScoreWeights) -> Vec<RankedGateway> {
    let mut ranked: Vec<RankedGateway> = candidates
        .iter()
        .map(|candidate| score_gateway(candidate, weights))
        .collect();

    ranked.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    ranked
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateways::GatewayConfig;
    use crate::scoring::types::{GatewayCandidate, ScoreInputs, ScoreWeights};

    #[test]
    fn rank_prefers_high_success_and_low_latency() {
        let weights = ScoreWeights {
            success_rate_weight: 0.35,
            latency_weight: 0.25,
            method_affinity_weight: 0.15,
            bank_affinity_weight: 0.12,
            amount_fit_weight: 0.08,
            time_weight: 0.05,
        };

        let g1 = GatewayCandidate {
            gateway: GatewayConfig {
                gateway_id: "g1".to_string(),
                gateway_name: "g1".to_string(),
                adapter_type: "MOCK".to_string(),
                is_enabled: true,
                priority: 1,
                supported_methods: vec!["UPI".to_string()],
                timeout_ms: 1000,
                mock_behavior: None,
            },
            inputs: ScoreInputs {
                success_rate: 0.95,
                p95_latency_ms: 800,
                method_affinity: 0.8,
                bank_affinity: 1.0,
                amount_fit: 0.8,
                time_multiplier: 1.0,
            },
        };

        let g2 = GatewayCandidate {
            gateway: GatewayConfig {
                gateway_id: "g2".to_string(),
                gateway_name: "g2".to_string(),
                adapter_type: "MOCK".to_string(),
                is_enabled: true,
                priority: 1,
                supported_methods: vec!["UPI".to_string()],
                timeout_ms: 1000,
                mock_behavior: None,
            },
            inputs: ScoreInputs {
                success_rate: 0.8,
                p95_latency_ms: 2200,
                method_affinity: 0.7,
                bank_affinity: 0.5,
                amount_fit: 0.7,
                time_multiplier: 1.0,
            },
        };

        let ranked = rank_gateways(&[g1, g2], &weights);
        assert_eq!(ranked[0].gateway_id, "g1");
    }
}

use crate::gateways::GatewayConfig;

#[derive(Debug, Clone)]
pub struct ScoreInputs {
    pub success_rate: f64,
    pub p95_latency_ms: i32,
    pub method_affinity: f64,
    pub bank_affinity: f64,
    pub amount_fit: f64,
    pub time_multiplier: f64,
}

#[derive(Debug, Clone)]
pub struct ScoreWeights {
    pub success_rate_weight: f64,
    pub latency_weight: f64,
    pub method_affinity_weight: f64,
    pub bank_affinity_weight: f64,
    pub amount_fit_weight: f64,
    pub time_weight: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScoreBreakdown {
    pub success_rate_score: f64,
    pub latency_score: f64,
    pub method_affinity: f64,
    pub bank_affinity: f64,
    pub amount_fit: f64,
    pub time_weight: f64,
    pub final_score: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RankedGateway {
    pub gateway_id: String,
    pub score: f64,
    pub breakdown: ScoreBreakdown,
}

#[derive(Debug, Clone)]
pub struct ScoringContext {
    pub payment_method: String,
    pub issuing_bank: String,
    pub amount_bucket: String,
}

#[derive(Debug, Clone)]
pub struct GatewayCandidate {
    pub gateway: GatewayConfig,
    pub inputs: ScoreInputs,
}

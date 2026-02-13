CREATE TABLE IF NOT EXISTS scoring_config (
    config_id TEXT PRIMARY KEY,
    success_rate_weight DOUBLE PRECISION NOT NULL,
    latency_weight DOUBLE PRECISION NOT NULL,
    method_affinity_weight DOUBLE PRECISION NOT NULL,
    bank_affinity_weight DOUBLE PRECISION NOT NULL,
    amount_fit_weight DOUBLE PRECISION NOT NULL,
    time_weight DOUBLE PRECISION NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO scoring_config (
    config_id,
    success_rate_weight,
    latency_weight,
    method_affinity_weight,
    bank_affinity_weight,
    amount_fit_weight,
    time_weight
)
VALUES ('default', 0.35, 0.25, 0.15, 0.12, 0.08, 0.05)
ON CONFLICT (config_id) DO NOTHING;

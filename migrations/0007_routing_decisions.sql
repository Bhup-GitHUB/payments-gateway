CREATE TABLE IF NOT EXISTS routing_decisions (
    payment_id UUID PRIMARY KEY,
    selected_gateway TEXT NOT NULL,
    selected_score DOUBLE PRECISION NOT NULL,
    runner_up_gateway TEXT NULL,
    runner_up_score DOUBLE PRECISION NULL,
    strategy TEXT NOT NULL,
    reason_summary TEXT NOT NULL,
    score_breakdown_json JSONB NOT NULL,
    ranked_gateways_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

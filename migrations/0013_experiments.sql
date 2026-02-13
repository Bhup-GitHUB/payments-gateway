CREATE TABLE IF NOT EXISTS experiments (
    experiment_id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    status TEXT NOT NULL,
    traffic_control_pct INT NOT NULL,
    traffic_treatment_pct INT NOT NULL,
    treatment_gateway TEXT NOT NULL,
    start_date TIMESTAMPTZ NOT NULL,
    end_date TIMESTAMPTZ NULL,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS experiment_variants (
    experiment_id UUID NOT NULL,
    variant_name TEXT NOT NULL,
    traffic_pct INT NOT NULL,
    override_gateway TEXT NULL,
    PRIMARY KEY (experiment_id, variant_name)
);

CREATE TABLE IF NOT EXISTS experiment_filters (
    experiment_id UUID NOT NULL,
    payment_method TEXT NULL,
    min_amount_minor BIGINT NULL,
    max_amount_minor BIGINT NULL,
    merchant_id TEXT NULL,
    amount_bucket TEXT NULL,
    PRIMARY KEY (experiment_id)
);

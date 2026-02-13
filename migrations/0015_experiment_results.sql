CREATE TABLE IF NOT EXISTS experiment_results (
    experiment_id UUID NOT NULL,
    variant TEXT NOT NULL,
    date_hour TIMESTAMPTZ NOT NULL,
    total_requests BIGINT NOT NULL,
    successful_requests BIGINT NOT NULL,
    failed_requests BIGINT NOT NULL,
    avg_latency_ms INT NOT NULL,
    p95_latency_ms INT NOT NULL,
    total_revenue_minor BIGINT NOT NULL,
    PRIMARY KEY (experiment_id, variant, date_hour)
);

CREATE INDEX IF NOT EXISTS idx_experiment_results_lookup
ON experiment_results(experiment_id, date_hour DESC);

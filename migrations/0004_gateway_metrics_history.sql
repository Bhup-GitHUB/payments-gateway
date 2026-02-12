CREATE TABLE IF NOT EXISTS gateway_metrics (
    snapshot_minute TIMESTAMPTZ NOT NULL,
    gateway_name TEXT NOT NULL,
    payment_method TEXT NOT NULL,
    issuing_bank TEXT NOT NULL,
    window_size_minutes INT NOT NULL,
    success_rate DOUBLE PRECISION NOT NULL,
    timeout_rate DOUBLE PRECISION NOT NULL,
    avg_latency_ms INT NOT NULL,
    p50_latency_ms INT NOT NULL,
    p95_latency_ms INT NOT NULL,
    p99_latency_ms INT NOT NULL,
    total_requests BIGINT NOT NULL,
    failed_requests BIGINT NOT NULL,
    timeout_requests BIGINT NOT NULL,
    error_counts_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (snapshot_minute, gateway_name, payment_method, issuing_bank, window_size_minutes)
);

CREATE INDEX IF NOT EXISTS idx_gateway_metrics_lookup
ON gateway_metrics (gateway_name, payment_method, issuing_bank, window_size_minutes, snapshot_minute DESC);

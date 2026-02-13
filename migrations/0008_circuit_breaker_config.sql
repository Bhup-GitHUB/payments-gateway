CREATE TABLE IF NOT EXISTS circuit_breaker_config (
    gateway_id TEXT NOT NULL,
    payment_method TEXT NOT NULL,
    failure_rate_threshold_2m DOUBLE PRECISION NOT NULL,
    consecutive_failure_threshold INT NOT NULL,
    timeout_rate_threshold_5m DOUBLE PRECISION NOT NULL,
    cooldown_seconds INT NOT NULL,
    half_open_probe_ratio DOUBLE PRECISION NOT NULL,
    half_open_min_probe_count INT NOT NULL,
    half_open_success_rate_close DOUBLE PRECISION NOT NULL,
    half_open_consecutive_success_close INT NOT NULL,
    half_open_consecutive_failure_reopen INT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (gateway_id, payment_method)
);

INSERT INTO circuit_breaker_config (
    gateway_id,
    payment_method,
    failure_rate_threshold_2m,
    consecutive_failure_threshold,
    timeout_rate_threshold_5m,
    cooldown_seconds,
    half_open_probe_ratio,
    half_open_min_probe_count,
    half_open_success_rate_close,
    half_open_consecutive_success_close,
    half_open_consecutive_failure_reopen
)
SELECT gateway_id, method, 0.40, 10, 0.50, 30, 0.10, 5, 0.80, 5, 3
FROM (
    SELECT gateway_id, unnest(ARRAY['UPI','CARD','NETBANKING']) AS method FROM gateways_config
) g
ON CONFLICT (gateway_id, payment_method) DO NOTHING;

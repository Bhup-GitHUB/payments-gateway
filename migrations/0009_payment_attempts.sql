CREATE TABLE IF NOT EXISTS payment_attempts (
    id BIGSERIAL PRIMARY KEY,
    payment_id UUID NOT NULL,
    attempt_number INT NOT NULL,
    gateway_used TEXT NOT NULL,
    status TEXT NOT NULL,
    error_code TEXT NULL,
    latency_ms INT NOT NULL,
    circuit_breaker_state TEXT NULL,
    fallback_reason TEXT NULL,
    transaction_ref TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (payment_id, attempt_number)
);

CREATE INDEX IF NOT EXISTS idx_payment_attempts_payment_id
ON payment_attempts(payment_id, attempt_number);

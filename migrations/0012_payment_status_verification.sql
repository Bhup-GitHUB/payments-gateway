CREATE TABLE IF NOT EXISTS payment_status_verification (
    payment_id UUID PRIMARY KEY,
    gateway_id TEXT NOT NULL,
    next_check_at TIMESTAMPTZ NOT NULL,
    attempts INT NOT NULL DEFAULT 0,
    status TEXT NOT NULL,
    last_response JSONB NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_payment_status_verification_next_check
ON payment_status_verification(status, next_check_at);

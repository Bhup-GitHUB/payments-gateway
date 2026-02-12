CREATE TABLE IF NOT EXISTS payment_events_outbox (
    id BIGSERIAL PRIMARY KEY,
    payment_id UUID NOT NULL,
    event_type TEXT NOT NULL,
    payload_json JSONB NOT NULL,
    status TEXT NOT NULL,
    attempts INT NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    published_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (payment_id, event_type)
);

CREATE INDEX IF NOT EXISTS idx_payment_events_outbox_status_next_attempt
ON payment_events_outbox(status, next_attempt_at);

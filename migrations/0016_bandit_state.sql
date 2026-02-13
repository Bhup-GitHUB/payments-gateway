CREATE TABLE IF NOT EXISTS bandit_policy (
    segment TEXT PRIMARY KEY,
    enabled BOOLEAN NOT NULL DEFAULT false,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS bandit_state (
    segment TEXT NOT NULL,
    gateway_id TEXT NOT NULL,
    alpha DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    beta DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (segment, gateway_id)
);

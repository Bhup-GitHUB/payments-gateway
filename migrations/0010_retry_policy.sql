CREATE TABLE IF NOT EXISTS retry_policy (
    merchant_id TEXT PRIMARY KEY,
    max_attempts INT NOT NULL DEFAULT 3,
    latency_budget_ms INT NOT NULL DEFAULT 10000,
    retry_on_timeout BOOLEAN NOT NULL DEFAULT false,
    enabled BOOLEAN NOT NULL DEFAULT true,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

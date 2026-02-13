CREATE TABLE IF NOT EXISTS experiment_assignments (
    experiment_id UUID NOT NULL,
    customer_id TEXT NOT NULL,
    variant TEXT NOT NULL,
    bucket INT NOT NULL,
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (experiment_id, customer_id)
);

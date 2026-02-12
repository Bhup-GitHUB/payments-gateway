CREATE TABLE IF NOT EXISTS gateways_config (
    gateway_id TEXT PRIMARY KEY,
    gateway_name TEXT NOT NULL,
    adapter_type TEXT NOT NULL,
    is_enabled BOOLEAN NOT NULL DEFAULT true,
    priority INT NOT NULL,
    supported_methods TEXT[] NOT NULL,
    timeout_ms INT NOT NULL DEFAULT 2500,
    mock_behavior TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO gateways_config (
    gateway_id,
    gateway_name,
    adapter_type,
    is_enabled,
    priority,
    supported_methods,
    timeout_ms,
    mock_behavior
) VALUES
    ('razorpay_real', 'Razorpay', 'RAZORPAY', true, 1, ARRAY['UPI','CARD','NETBANKING'], 2500, NULL),
    ('hdfc_mock', 'HDFC', 'MOCK', true, 2, ARRAY['UPI','CARD','NETBANKING'], 1200, 'ALWAYS_SUCCESS'),
    ('icici_mock', 'ICICI', 'MOCK', true, 3, ARRAY['UPI','CARD','NETBANKING'], 1200, 'ALWAYS_FAILURE'),
    ('axis_mock', 'Axis', 'MOCK', true, 4, ARRAY['UPI','CARD','NETBANKING'], 1200, 'ALWAYS_TIMEOUT')
ON CONFLICT (gateway_id) DO NOTHING;

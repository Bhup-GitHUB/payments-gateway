CREATE TABLE IF NOT EXISTS gateway_error_classification (
    gateway_id TEXT NOT NULL,
    error_code TEXT NOT NULL,
    retryable BOOLEAN NOT NULL,
    timeout_like BOOLEAN NOT NULL DEFAULT false,
    non_retryable_user_error BOOLEAN NOT NULL DEFAULT false,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (gateway_id, error_code)
);

INSERT INTO gateway_error_classification (gateway_id, error_code, retryable, timeout_like, non_retryable_user_error) VALUES
('razorpay_real', 'TIMEOUT', false, true, false),
('razorpay_real', 'NETWORK_ERROR', true, false, false),
('razorpay_real', 'HTTP_500', true, false, false),
('razorpay_real', 'HTTP_503', true, false, false),
('razorpay_real', 'INSUFFICIENT_FUNDS', false, false, true),
('hdfc_mock', 'MOCK_TIMEOUT', false, true, false),
('hdfc_mock', 'MOCK_DECLINED', true, false, false),
('icici_mock', 'MOCK_TIMEOUT', false, true, false),
('icici_mock', 'MOCK_DECLINED', true, false, false),
('axis_mock', 'MOCK_TIMEOUT', false, true, false),
('axis_mock', 'MOCK_DECLINED', true, false, false)
ON CONFLICT (gateway_id, error_code) DO NOTHING;

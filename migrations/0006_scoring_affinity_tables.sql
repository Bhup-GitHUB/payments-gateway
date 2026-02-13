CREATE TABLE IF NOT EXISTS gateway_method_affinity (
    gateway_id TEXT NOT NULL,
    payment_method TEXT NOT NULL,
    score DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (gateway_id, payment_method)
);

CREATE TABLE IF NOT EXISTS gateway_amount_fit (
    gateway_id TEXT NOT NULL,
    amount_bucket TEXT NOT NULL,
    score DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (gateway_id, amount_bucket)
);

CREATE TABLE IF NOT EXISTS gateway_time_penalty (
    gateway_id TEXT NOT NULL,
    hour_of_day INT NOT NULL,
    day_of_month INT NULL,
    multiplier DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (gateway_id, hour_of_day, day_of_month)
);

CREATE TABLE IF NOT EXISTS bin_bank_map (
    bin_prefix TEXT PRIMARY KEY,
    bank_code TEXT NOT NULL
);

INSERT INTO gateway_method_affinity (gateway_id, payment_method, score) VALUES
('hdfc_mock', 'UPI', 0.9),
('hdfc_mock', 'CARD', 0.8),
('hdfc_mock', 'NETBANKING', 0.7),
('icici_mock', 'UPI', 0.75),
('icici_mock', 'CARD', 0.8),
('icici_mock', 'NETBANKING', 0.75),
('axis_mock', 'UPI', 0.7),
('axis_mock', 'CARD', 0.7),
('axis_mock', 'NETBANKING', 0.7),
('razorpay_real', 'UPI', 1.0),
('razorpay_real', 'CARD', 0.6),
('razorpay_real', 'NETBANKING', 0.7)
ON CONFLICT (gateway_id, payment_method) DO NOTHING;

INSERT INTO gateway_amount_fit (gateway_id, amount_bucket, score) VALUES
('razorpay_real', 'lt_500', 1.0),
('hdfc_mock', 'lt_500', 0.7),
('icici_mock', 'lt_500', 0.7),
('axis_mock', 'lt_500', 0.7),
('razorpay_real', '500_2000', 0.8),
('hdfc_mock', '500_2000', 0.8),
('icici_mock', '500_2000', 0.8),
('axis_mock', '500_2000', 0.8),
('hdfc_mock', '2000_10000', 1.0),
('icici_mock', '2000_10000', 0.9),
('razorpay_real', '2000_10000', 0.6),
('axis_mock', '2000_10000', 0.7),
('hdfc_mock', 'gt_10000', 1.0),
('icici_mock', 'gt_10000', 0.9),
('razorpay_real', 'gt_10000', 0.6),
('axis_mock', 'gt_10000', 0.7)
ON CONFLICT (gateway_id, amount_bucket) DO NOTHING;

INSERT INTO gateway_time_penalty (gateway_id, hour_of_day, day_of_month, multiplier) VALUES
('hdfc_mock', 12, NULL, 0.9),
('hdfc_mock', 13, NULL, 0.9),
('hdfc_mock', 18, NULL, 0.9),
('hdfc_mock', 19, NULL, 0.9),
('hdfc_mock', 12, 10, 0.8),
('hdfc_mock', 13, 10, 0.8),
('hdfc_mock', 12, 25, 0.8),
('hdfc_mock', 13, 25, 0.8)
ON CONFLICT (gateway_id, hour_of_day, day_of_month) DO NOTHING;

INSERT INTO bin_bank_map (bin_prefix, bank_code) VALUES
('411111', 'HDFC'),
('555555', 'ICICI'),
('444433', 'AXIS')
ON CONFLICT (bin_prefix) DO NOTHING;

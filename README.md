# payments-gateway

Rust + Axum payment API implementing Phase 1 and Phase 2 foundations:
- `POST /payments` with idempotency and unified response
- `GET /gateways` and `PATCH /gateways/:gateway_id`
- Phase 2 round-robin routing across enabled gateways
- PostgreSQL persistence for payments and gateway configs

## Required env
- `DATABASE_URL` (default: `postgres://postgres:postgres@localhost:5432/payments_gateway`)
- `RAZORPAY_KEY_ID`
- `RAZORPAY_KEY_SECRET`
- `RAZORPAY_BASE_URL` (default: `https://api.razorpay.com`)
- `GATEWAY_TIMEOUT_MS` (default: `2500`)
- `BIND_ADDR` (default: `0.0.0.0:3000`)

## Run
```bash
cargo run
```

## Example request
```bash
curl -X POST http://localhost:3000/payments \
  -H 'Content-Type: application/json' \
  -H 'Idempotency-Key: demo-123' \
  -d '{
    "amount_minor": 1000,
    "currency": "INR",
    "payment_method": "UPI",
    "merchant_id": "m_001",
    "instrument": {
      "type": "UPI",
      "vpa": "test@okhdfcbank"
    }
  }'
```

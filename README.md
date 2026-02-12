# payments-gateway

Rust + Axum payment API implementing Phase 1, Phase 2, and Phase 3 foundations.

## APIs
- `POST /payments`
- `GET /gateways`
- `PATCH /gateways/:gateway_id`
- `GET /metrics/gateways/:gateway_name`

## Phase 3 architecture
- `POST /payments` persists payment and outbox event in one DB transaction.
- API background relay publishes outbox events to Redis Stream.
- `metrics_worker` consumes stream events.
- Worker computes 1m/5m/15m/60m metrics and writes hot metrics to Redis.
- Worker writes historical snapshots to Postgres `gateway_metrics`.

## Required env
- `DATABASE_URL` (default: `postgres://postgres:postgres@localhost:5432/payments_gateway`)
- `BIND_ADDR` (default: `0.0.0.0:3000`)
- `RAZORPAY_KEY_ID`
- `RAZORPAY_KEY_SECRET`
- `RAZORPAY_BASE_URL` (default: `https://api.razorpay.com`)
- `GATEWAY_TIMEOUT_MS` (default: `2500`)
- `REDIS_URL` (default: `redis://127.0.0.1:6379/`)
- `METRICS_STREAM_KEY` (default: `payments:events:v1`)
- `METRICS_STREAM_GROUP` (default: `metrics-agg-v1`)
- `METRICS_CONSUMER_NAME` (worker only, default: `metrics-worker-1`)

## Run API
```bash
cargo run
```

## Run worker
```bash
cargo run --bin metrics_worker
```

## Example payment request
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

## Example metrics request
```bash
curl "http://localhost:3000/metrics/gateways/razorpay_real?window=5m"
```

# payments-gateway

Rust + Axum payment API implementing Phase 1 to Phase 5 foundations.

## APIs
- `POST /payments`
- `GET /payments/:payment_id/routing-decision`
- `GET /gateways`
- `PATCH /gateways/:gateway_id`
- `GET /metrics/gateways/:gateway_name`
- `GET /scoring/debug`
- `GET /circuit-breaker/status`
- `POST /circuit-breaker/force-open/:gateway/:method`
- `POST /circuit-breaker/force-close/:gateway/:method`

## Routing flow
- Build payment context.
- Load enabled gateways for payment method.
- Read live metrics from Redis.
- Score each gateway with weighted scoring engine.
- Filter candidates through circuit breaker state.
- Execute selected gateway.
- Persist payment, outbox event, and routing decision.

## Metrics and event pipeline
- `POST /payments` writes payment + outbox in one DB transaction.
- API relay publishes outbox events to Redis Stream.
- `metrics_worker` consumes events and writes:
  - hot metrics in Redis
  - historical snapshots in Postgres `gateway_metrics`

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

## Example routing decision request
```bash
curl "http://localhost:3000/payments/<payment_id>/routing-decision"
```

## Example scoring debug request
```bash
curl "http://localhost:3000/scoring/debug?amount_minor=250000&payment_method=UPI&issuing_bank=HDFC"
```

## Example circuit status request
```bash
curl "http://localhost:3000/circuit-breaker/status"
```

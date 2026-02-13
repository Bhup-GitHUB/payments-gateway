# payments-gateway

Rust + Axum payment API implementing Phase 1 to Phase 7 foundations with scoring, circuit breaker, fallback retries, experiments, and optimization controls.

## APIs
- `POST /payments`
- `GET /payments/:payment_id/routing-decision`
- `GET /payments/:payment_id/attempts`
- `GET /payments/:payment_id/status-verification`
- `GET /gateways`
- `PATCH /gateways/:gateway_id`
- `GET /metrics/gateways/:gateway_name`
- `GET /scoring/debug`
- `GET /circuit-breaker/status`
- `POST /circuit-breaker/force-open/:gateway/:method` (admin key)
- `POST /circuit-breaker/force-close/:gateway/:method` (admin key)
- `GET /retry-policy/:merchant_id`
- `PUT /retry-policy/:merchant_id` (admin key)
- `POST /experiments` (admin key)
- `GET /experiments`
- `GET /experiments/:id/results`
- `GET /experiments/:id/winner`
- `POST /experiments/:id/stop` (admin key)
- `POST /bandit/policy/:segment/enable` (admin key)
- `GET /bandit/state`
- `GET /ops/readiness`
- `GET /ops/liveness`

## Routing and reliability flow
- Build payment context.
- Load enabled gateways for payment method.
- Apply experiment override (deterministic by `customer_id`) when active.
- Score gateways with weighted scoring engine.
- Optionally reorder with feature-flagged Thompson sampling by segment.
- Apply circuit-breaker checks.
- Execute fallback retry chain under merchant policy and latency budget.
- On timeout, return `PENDING_VERIFICATION` and enqueue verification job.
- Persist payment, outbox event, payment attempts, and routing decision.

## Metrics and events
- Outbox relay publishes payment events to Redis Streams.
- `metrics_worker` consumes stream and updates:
  - hot metrics in Redis
  - historical snapshots in Postgres `gateway_metrics`
- `experiment_analyzer` computes significance recommendations.
- `payment_verifier` processes pending verification queue.

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
- `INTERNAL_API_KEY` (default: `dev-internal-key`)

## Run API
```bash
cargo run
```

## Run workers
```bash
cargo run --bin metrics_worker
cargo run --bin payment_verifier
cargo run --bin experiment_analyzer
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
    "customer_id": "cust_001",
    "instrument": {
      "type": "UPI",
      "vpa": "test@okhdfcbank"
    }
  }'
```

## Example admin request
```bash
curl -X PUT http://localhost:3000/retry-policy/m_001 \
  -H 'Content-Type: application/json' \
  -H 'X-Internal-Api-Key: dev-internal-key' \
  -d '{
    "merchant_id": "m_001",
    "max_attempts": 3,
    "latency_budget_ms": 10000,
    "retry_on_timeout": false,
    "enabled": true
  }'
```

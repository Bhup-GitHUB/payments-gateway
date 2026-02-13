# payments-gateway

Production-focused payment routing service in Rust + Axum.

This service processes payments across multiple gateways with:
- deterministic fallback and retry policies
- timeout-safe `PENDING_VERIFICATION` flow
- real-time metrics pipeline
- scoring-based gateway selection
- circuit breaker protection
- experiment framework (A/B)
- feature-flagged Thompson sampling pilot
- admin controls, rate limits, and operational readiness endpoints

## System design

```mermaid
flowchart LR
    A[Client] --> B[POST /payments]
    B --> C[PaymentService]
    C --> D[Build Context]
    D --> E[Experiment Assignment]
    E --> F[Scoring Engine]
    F --> G[Bandit Reorder if enabled]
    G --> H[Circuit Check]
    H --> I[Retry Orchestrator]
    I --> J[Gateway Adapter]
    J --> K[Gateway Response]
    K --> L[Persist Payment + Attempts + Routing Decision]
    L --> M[Outbox Event]
    M --> N[Redis Stream]
    N --> O[Metrics Worker]
    O --> P[Redis Hot Metrics]
    O --> Q[Postgres Historical Metrics]

    K --> R{Status}
    R -->|SUCCESS| S[Return Success]
    R -->|FAILURE| T[Classify Error and Retry if allowed]
    R -->|TIMEOUT| U[Mark PENDING_VERIFICATION]
    U --> V[Verification Queue]
    V --> W[payment_verifier]
```

## Request lifecycle

1. Validate request and idempotency key.
2. Build payment context from amount, method, instrument, merchant, and headers.
3. Resolve active experiment and deterministic variant using `customer_id`.
4. Score gateways from live metrics and affinity configuration.
5. Optionally reorder candidates with bandit for enabled segments.
6. Enforce circuit breaker state before each attempt.
7. Execute retry chain using merchant retry policy and total latency budget.
8. Handle timeout as `PENDING_VERIFICATION` and enqueue verification instead of unsafe immediate retry.
9. Persist payment record, attempt trace, routing decision, and outbox event.
10. Return unified payment response.

## Core API

### Payments
- `POST /payments`
- `GET /payments/:payment_id/routing-decision`
- `GET /payments/:payment_id/attempts`
- `GET /payments/:payment_id/status-verification`

### Gateway and metrics
- `GET /gateways`
- `PATCH /gateways/:gateway_id`
- `GET /metrics/gateways/:gateway_name`
- `GET /scoring/debug`

### Circuit breaker
- `GET /circuit-breaker/status`
- `POST /circuit-breaker/force-open/:gateway/:method` (admin)
- `POST /circuit-breaker/force-close/:gateway/:method` (admin)

### Retry policy
- `GET /retry-policy/:merchant_id`
- `PUT /retry-policy/:merchant_id` (admin)

### Experiments
- `POST /experiments` (admin)
- `GET /experiments`
- `GET /experiments/:id/results`
- `GET /experiments/:id/winner`
- `POST /experiments/:id/stop` (admin)

### Bandit
- `POST /bandit/policy/:segment/enable` (admin)
- `POST /bandit/policy/:segment/disable` (admin)
- `GET /bandit/state`

### Ops
- `GET /ops/readiness`
- `GET /ops/liveness`

## Components

- `payment_service`: main routing, scoring, retry, and persistence orchestration.
- `metrics_worker`: consumes Redis stream and updates hot + historical gateway metrics.
- `payment_verifier`: reconciles timeout cases in verification queue.
- `experiment_analyzer`: computes significance and auto-pauses harmful treatments via guardrails.

## Security and controls

- Admin mutation routes require `X-Internal-Api-Key`.
- Global IP-based rate limit middleware to protect service and gateways.
- Circuit breaker events and reliability signals can emit to webhook subscriptions.

## Environment variables

- `DATABASE_URL` default `postgres://postgres:postgres@localhost:5432/payments_gateway`
- `BIND_ADDR` default `0.0.0.0:3000`
- `REDIS_URL` default `redis://127.0.0.1:6379/`
- `METRICS_STREAM_KEY` default `payments:events:v1`
- `METRICS_STREAM_GROUP` default `metrics-agg-v1`
- `METRICS_CONSUMER_NAME` default `metrics-worker-1` (worker)
- `RAZORPAY_KEY_ID`
- `RAZORPAY_KEY_SECRET`
- `RAZORPAY_BASE_URL` default `https://api.razorpay.com`
- `GATEWAY_TIMEOUT_MS` default `2500`
- `INTERNAL_API_KEY` default `dev-internal-key`
- `EXPERIMENT_GUARDRAIL_MIN_SAMPLES` default `100`
- `EXPERIMENT_GUARDRAIL_MAX_SUCCESS_DROP` default `0.05`
- `EXPERIMENT_GUARDRAIL_MAX_LATENCY_MULTIPLIER` default `1.5`

## Run

```bash
cargo run
```

## Run background workers

```bash
cargo run --bin metrics_worker
cargo run --bin payment_verifier
cargo run --bin experiment_analyzer
```

## Example create payment

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

## Example update retry policy

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

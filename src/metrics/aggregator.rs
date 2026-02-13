use crate::metrics::event::PaymentEvent;
use crate::metrics::window::{minute_epoch, percentile, MinuteBucket};
use crate::domain::payment::PaymentStatus;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct MetricKey {
    pub gateway: String,
    pub method: String,
    pub bank: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMetric {
    pub success_rate: f64,
    pub timeout_rate: f64,
    pub avg_latency_ms: i32,
    pub p50_latency_ms: i32,
    pub p95_latency_ms: i32,
    pub p99_latency_ms: i32,
    pub total_requests: u64,
    pub failed_requests: u64,
    pub timeout_requests: u64,
    pub error_counts: HashMap<String, u64>,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Default)]
pub struct SlidingMetrics {
    buckets: HashMap<MetricKey, BTreeMap<i64, MinuteBucket>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::payment::PaymentStatus;
    use crate::metrics::event::PaymentEvent;
    use chrono::TimeZone;
    use uuid::Uuid;

    #[test]
    fn computes_window_metric() {
        let mut s = SlidingMetrics::default();
        let ts = chrono::Utc.timestamp_opt(1_700_000_000, 0).single().unwrap();
        let key = MetricKey {
            gateway: "g1".to_string(),
            method: "UPI".to_string(),
            bank: "HDFC".to_string(),
        };

        s.ingest(&PaymentEvent {
            payment_id: Uuid::new_v4(),
            gateway_used: "g1".to_string(),
            payment_method: "UPI".to_string(),
            issuing_bank: "HDFC".to_string(),
            amount_bucket: "lt_500".to_string(),
            status: PaymentStatus::Success,
            latency_ms: 100,
            error_code: None,
            timestamp: ts,
        });
        s.ingest(&PaymentEvent {
            payment_id: Uuid::new_v4(),
            gateway_used: "g1".to_string(),
            payment_method: "UPI".to_string(),
            issuing_bank: "HDFC".to_string(),
            amount_bucket: "lt_500".to_string(),
            status: PaymentStatus::Failure,
            latency_ms: 200,
            error_code: Some("DECLINED".to_string()),
            timestamp: ts,
        });

        let m = s.compute(&key, 5, ts).unwrap();
        assert_eq!(m.total_requests, 2);
        assert_eq!(m.failed_requests, 1);
        assert!(m.success_rate > 0.49 && m.success_rate < 0.51);
    }
}

impl SlidingMetrics {
    pub fn ingest(&mut self, event: &PaymentEvent) {
        let key = MetricKey {
            gateway: event.gateway_used.clone(),
            method: event.payment_method.clone(),
            bank: event.issuing_bank.clone(),
        };
        let minute = minute_epoch(event.timestamp);
        let bucket_map = self.buckets.entry(key).or_default();
        let bucket = bucket_map
            .entry(minute)
            .or_insert_with(|| MinuteBucket::new(minute));

        bucket.total += 1;
        bucket.latencies.push(event.latency_ms);
        match event.status {
            PaymentStatus::Success => {}
            PaymentStatus::Failure => {
                bucket.failed += 1;
                if let Some(code) = &event.error_code {
                    *bucket.error_counts.entry(code.clone()).or_insert(0) += 1;
                }
            }
            PaymentStatus::Timeout => {
                bucket.failed += 1;
                bucket.timeout += 1;
                if let Some(code) = &event.error_code {
                    *bucket.error_counts.entry(code.clone()).or_insert(0) += 1;
                }
            }
            PaymentStatus::PendingVerification => {
                bucket.failed += 1;
                bucket.timeout += 1;
                if let Some(code) = &event.error_code {
                    *bucket.error_counts.entry(code.clone()).or_insert(0) += 1;
                }
            }
        }

        let floor = minute - (59 * 60);
        bucket_map.retain(|m, _| *m >= floor);
    }

    pub fn keys(&self) -> Vec<MetricKey> {
        self.buckets.keys().cloned().collect()
    }

    pub fn compute(&self, key: &MetricKey, window_minutes: i64, now: chrono::DateTime<chrono::Utc>) -> Option<AggregatedMetric> {
        let start = minute_epoch(now) - ((window_minutes - 1) * 60);
        let end = minute_epoch(now);
        let map = self.buckets.get(key)?;

        let mut total: u64 = 0;
        let mut failed: u64 = 0;
        let mut timeout: u64 = 0;
        let mut latencies: Vec<i32> = Vec::new();
        let mut error_counts: HashMap<String, u64> = HashMap::new();

        for bucket in map.values() {
            if bucket.minute < start || bucket.minute > end {
                continue;
            }
            total += bucket.total;
            failed += bucket.failed;
            timeout += bucket.timeout;
            latencies.extend(bucket.latencies.iter().copied());
            for (code, count) in &bucket.error_counts {
                *error_counts.entry(code.clone()).or_insert(0) += count;
            }
        }

        if total == 0 {
            return None;
        }

        latencies.sort_unstable();
        let sum: i64 = latencies.iter().map(|x| *x as i64).sum();
        let avg_latency_ms = (sum / latencies.len() as i64) as i32;

        Some(AggregatedMetric {
            success_rate: (total - failed) as f64 / total as f64,
            timeout_rate: timeout as f64 / total as f64,
            avg_latency_ms,
            p50_latency_ms: percentile(&latencies, 0.50),
            p95_latency_ms: percentile(&latencies, 0.95),
            p99_latency_ms: percentile(&latencies, 0.99),
            total_requests: total,
            failed_requests: failed,
            timeout_requests: timeout,
            error_counts,
            generated_at: chrono::Utc::now(),
        })
    }
}

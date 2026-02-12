use chrono::{DateTime, Timelike, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MinuteBucket {
    pub minute: i64,
    pub total: u64,
    pub failed: u64,
    pub timeout: u64,
    pub latencies: Vec<i32>,
    pub error_counts: HashMap<String, u64>,
}

impl MinuteBucket {
    pub fn new(minute: i64) -> Self {
        Self {
            minute,
            total: 0,
            failed: 0,
            timeout: 0,
            latencies: Vec::new(),
            error_counts: HashMap::new(),
        }
    }
}

pub fn minute_epoch(ts: DateTime<Utc>) -> i64 {
    ts.timestamp() - (ts.second() as i64)
}

pub fn percentile(sorted: &[i32], p: f64) -> i32 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx]
}

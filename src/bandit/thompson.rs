use rand::thread_rng;
use rand_distr::{Beta, Distribution};

pub fn sample(alpha: f64, beta: f64) -> f64 {
    if let Ok(dist) = Beta::new(alpha.max(0.001), beta.max(0.001)) {
        dist.sample(&mut thread_rng())
    } else {
        0.5
    }
}

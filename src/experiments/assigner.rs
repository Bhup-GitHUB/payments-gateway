use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize)]
pub struct Assignment {
    pub variant: String,
    pub bucket: i32,
}

pub fn assign_variant(customer_id: &str, experiment_id: Uuid, control_pct: i32) -> Assignment {
    let mut hasher = Sha256::new();
    hasher.update(customer_id.as_bytes());
    hasher.update(experiment_id.as_bytes());
    let hash = hasher.finalize();

    let bucket = ((hash[0] as u16 * 256 + hash[1] as u16) % 100) as i32;
    let variant = if bucket < control_pct { "control" } else { "treatment" };

    Assignment {
        variant: variant.to_string(),
        bucket,
    }
}

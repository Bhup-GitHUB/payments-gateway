use crate::domain::experiment::ExperimentFilter;

#[derive(Debug, Clone)]
pub struct MatchInput {
    pub payment_method: String,
    pub amount_minor: i64,
    pub merchant_id: String,
    pub amount_bucket: String,
}

pub fn matches(filter: &ExperimentFilter, input: &MatchInput) -> bool {
    if let Some(method) = &filter.payment_method {
        if method.to_uppercase() != input.payment_method.to_uppercase() {
            return false;
        }
    }

    if let Some(min) = filter.min_amount_minor {
        if input.amount_minor < min {
            return false;
        }
    }

    if let Some(max) = filter.max_amount_minor {
        if input.amount_minor > max {
            return false;
        }
    }

    if let Some(merchant) = &filter.merchant_id {
        if merchant != &input.merchant_id {
            return false;
        }
    }

    if let Some(bucket) = &filter.amount_bucket {
        if bucket != &input.amount_bucket {
            return false;
        }
    }

    true
}

use anyhow::Result;
use sqlx::{PgPool, Row};

#[derive(Clone)]
pub struct ErrorClassificationRepo {
    pub pool: PgPool,
}

#[derive(Debug, Clone)]
pub struct ErrorClass {
    pub retryable: bool,
    pub timeout_like: bool,
    pub non_retryable_user_error: bool,
}

impl ErrorClassificationRepo {
    pub async fn classify(&self, gateway_id: &str, error_code: &str) -> Result<ErrorClass> {
        let row = sqlx::query(
            "SELECT retryable, timeout_like, non_retryable_user_error FROM gateway_error_classification WHERE gateway_id=$1 AND error_code=$2",
        )
        .bind(gateway_id)
        .bind(error_code)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(ErrorClass {
                retryable: row.get("retryable"),
                timeout_like: row.get("timeout_like"),
                non_retryable_user_error: row.get("non_retryable_user_error"),
            })
        } else {
            Ok(ErrorClass {
                retryable: false,
                timeout_like: false,
                non_retryable_user_error: false,
            })
        }
    }
}

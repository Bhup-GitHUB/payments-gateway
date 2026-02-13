use crate::domain::payment::PaymentStatus;
use crate::repo::error_classification_repo::ErrorClass;
use crate::repo::retry_policy_repo::RetryPolicy;

#[derive(Debug, Clone)]
pub enum RetryDirective {
    Success,
    Continue,
    FailNow,
    PendingVerification,
}

pub fn should_stop_for_budget(start: std::time::Instant, policy: &RetryPolicy) -> bool {
    start.elapsed().as_millis() as i32 >= policy.latency_budget_ms
}

pub fn attempt_limit(policy: &RetryPolicy) -> i32 {
    if policy.enabled {
        policy.max_attempts.max(0)
    } else {
        1
    }
}

pub fn classify_attempt_result(
    status: &PaymentStatus,
    error_class: Option<&ErrorClass>,
    retry_on_timeout: bool,
) -> RetryDirective {
    match status {
        PaymentStatus::Success => RetryDirective::Success,
        PaymentStatus::PendingVerification => RetryDirective::PendingVerification,
        PaymentStatus::Timeout => {
            if retry_on_timeout {
                RetryDirective::Continue
            } else {
                RetryDirective::PendingVerification
            }
        }
        PaymentStatus::Failure => {
            if let Some(class) = error_class {
                if class.non_retryable_user_error {
                    RetryDirective::FailNow
                } else if class.retryable {
                    RetryDirective::Continue
                } else {
                    RetryDirective::FailNow
                }
            } else {
                RetryDirective::FailNow
            }
        }
    }
}

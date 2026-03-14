use crate::errors::HuefyError;
use rand::Rng;
use std::future::Future;
use std::time::Duration;

/// Base delay in milliseconds for the first retry attempt.
const BASE_DELAY_MS: u64 = 1000;

/// Maximum delay in milliseconds between retry attempts.
const MAX_DELAY_MS: u64 = 30_000;

/// Executes an async operation with exponential backoff retry.
///
/// The closure `operation` is called up to `max_retries + 1` times. Between
/// attempts the delay doubles (with jitter) up to a maximum of 30 seconds.
/// Only *recoverable* errors trigger a retry.
pub async fn with_retry<F, Fut, T>(max_retries: u32, operation: F) -> Result<T, HuefyError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, HuefyError>>,
{
    let mut last_error: Option<HuefyError> = None;

    for attempt in 0..=max_retries {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(err) => {
                if !err.is_recoverable() || attempt == max_retries {
                    return Err(err);
                }

                // If the server sent a Retry-After value, honour it instead
                // of the default exponential backoff.
                let delay = match &err {
                    HuefyError::RateLimited {
                        retry_after: Some(secs),
                        ..
                    } => Duration::from_secs(*secs),
                    _ => calculate_delay(attempt),
                };

                tokio::time::sleep(delay).await;
                last_error = Some(err);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| HuefyError::Unknown {
        message: "Retry loop exited without result".to_string(),
        code: crate::errors::ErrorCode::Unknown,
        source: None,
    }))
}

/// Calculates the retry delay for the given attempt number.
///
/// Uses exponential backoff with a random jitter of up to 20% of the base
/// delay. The result is capped at 30 seconds.
pub fn calculate_delay(attempt: u32) -> Duration {
    let exponential = BASE_DELAY_MS.saturating_mul(2u64.saturating_pow(attempt));
    let capped = exponential.min(MAX_DELAY_MS);

    // Random jitter ±25% to prevent thundering herd. Result is capped at MAX_DELAY_MS.
    let jitter_factor = 0.75 + rand::thread_rng().gen::<f64>() * 0.5;
    Duration::from_millis(((capped as f64 * jitter_factor) as u64).min(MAX_DELAY_MS))
}

/// Parses a `Retry-After` header value into a `Duration`.
///
/// Supports both delta-seconds (`120`) and HTTP-date formats (only the
/// delta-seconds variant is implemented here; dates fall through to `None`).
pub fn parse_retry_after(value: &str) -> Option<Duration> {
    value
        .trim()
        .parse::<u64>()
        .ok()
        .map(Duration::from_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_delay_increases() {
        let d0 = calculate_delay(0);
        let d1 = calculate_delay(1);
        let d2 = calculate_delay(2);

        assert!(d1 > d0);
        assert!(d2 > d1);
    }

    #[test]
    fn test_calculate_delay_capped() {
        let d = calculate_delay(20);
        assert!(d <= Duration::from_secs(30)); // capped at MAX_DELAY_MS
    }

    #[test]
    fn test_parse_retry_after_seconds() {
        assert_eq!(parse_retry_after("120"), Some(Duration::from_secs(120)));
        assert_eq!(parse_retry_after("  5  "), Some(Duration::from_secs(5)));
    }

    #[test]
    fn test_parse_retry_after_invalid() {
        assert_eq!(parse_retry_after("not-a-number"), None);
    }

    #[tokio::test]
    async fn test_with_retry_succeeds_immediately() {
        let result = with_retry(3, || async { Ok::<_, HuefyError>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_with_retry_non_recoverable_fails_fast() {
        let result = with_retry(3, || async {
            Err::<(), _>(HuefyError::Auth {
                message: "bad key".to_string(),
                code: crate::errors::ErrorCode::Authentication,
            })
        })
        .await;

        assert!(result.is_err());
    }
}

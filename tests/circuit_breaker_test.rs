use huefy::errors::{ErrorCode, HuefyError};
use huefy::http::circuit_breaker::{CircuitBreaker, CircuitState};
use std::time::Duration;

#[tokio::test]
async fn test_circuit_breaker_closed_allows_requests() {
    let cb = CircuitBreaker::new(5, Duration::from_secs(30), 1);

    let result = cb.execute(|| async { Ok::<i32, HuefyError>(42) }).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
    assert_eq!(cb.state(), CircuitState::Closed);
}

#[tokio::test]
async fn test_circuit_breaker_opens_after_threshold() {
    let cb = CircuitBreaker::new(3, Duration::from_secs(30), 1);

    // Trip the circuit with 3 consecutive failures.
    for _ in 0..3 {
        let _ = cb
            .execute(|| async {
                Err::<(), _>(HuefyError::Network {
                    message: "connection refused".to_string(),
                    code: ErrorCode::Network,
                    source: None,
                })
            })
            .await;
    }

    assert_eq!(cb.state(), CircuitState::Open);

    // Next request should be rejected immediately.
    let result = cb.execute(|| async { Ok::<_, HuefyError>(()) }).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        HuefyError::CircuitBreakerOpen { .. } => {}
        other => panic!("Expected CircuitBreakerOpen, got {:?}", other),
    }
}

#[tokio::test]
async fn test_circuit_breaker_resets_on_success() {
    let cb = CircuitBreaker::new(3, Duration::from_secs(30), 1);

    // Accumulate 2 failures (below threshold).
    for _ in 0..2 {
        let _ = cb
            .execute(|| async {
                Err::<(), _>(HuefyError::Network {
                    message: "fail".to_string(),
                    code: ErrorCode::Network,
                    source: None,
                })
            })
            .await;
    }

    assert_eq!(cb.state(), CircuitState::Closed);

    // A success resets the failure count.
    let _ = cb.execute(|| async { Ok::<_, HuefyError>(()) }).await;
    assert_eq!(cb.state(), CircuitState::Closed);
}

#[tokio::test]
async fn test_non_recoverable_errors_do_not_trip_breaker() {
    let cb = CircuitBreaker::new(1, Duration::from_secs(30), 1);

    // An authentication error is non-recoverable and should NOT trip the
    // circuit.
    let _ = cb
        .execute(|| async {
            Err::<(), _>(HuefyError::Auth {
                message: "invalid key".to_string(),
                code: ErrorCode::Authentication,
            })
        })
        .await;

    assert_eq!(cb.state(), CircuitState::Closed);
}

#[tokio::test]
async fn test_half_open_transitions_to_closed() {
    let cb = CircuitBreaker::new(1, Duration::from_millis(50), 1);

    // Trip the circuit.
    let _ = cb
        .execute(|| async {
            Err::<(), _>(HuefyError::Network {
                message: "fail".to_string(),
                code: ErrorCode::Network,
                source: None,
            })
        })
        .await;

    assert_eq!(cb.state(), CircuitState::Open);

    // Wait for the reset timeout so the state transitions to HalfOpen.
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(cb.state(), CircuitState::HalfOpen);

    // A successful probe should close the circuit again.
    let result = cb.execute(|| async { Ok::<_, HuefyError>(()) }).await;
    assert!(result.is_ok());
    assert_eq!(cb.state(), CircuitState::Closed);
}

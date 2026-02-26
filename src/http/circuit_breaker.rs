use crate::errors::{ErrorCode, HuefyError};
use std::future::Future;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The state of the circuit breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// All requests are allowed through.
    Closed,
    /// Requests are blocked; the circuit tripped after too many failures.
    Open,
    /// A limited number of probe requests are allowed to test recovery.
    HalfOpen,
}

/// Internal mutable state protected by a `Mutex`.
#[derive(Debug)]
struct Inner {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<Instant>,
    half_open_in_flight: u32,
}

/// A circuit breaker that monitors outbound request failures and temporarily
/// halts traffic when a failure threshold is reached.
#[derive(Debug)]
pub struct CircuitBreaker {
    failure_threshold: u32,
    reset_timeout: Duration,
    half_open_max_requests: u32,
    inner: Mutex<Inner>,
}

impl CircuitBreaker {
    /// Creates a new `CircuitBreaker`.
    ///
    /// - `failure_threshold` -- consecutive failures before the circuit opens.
    /// - `reset_timeout` -- how long the circuit stays open before allowing
    ///   probes.
    /// - `half_open_max_requests` -- successful probes needed to fully close
    ///   the circuit.
    pub fn new(
        failure_threshold: u32,
        reset_timeout: Duration,
        half_open_max_requests: u32,
    ) -> Self {
        Self {
            failure_threshold,
            reset_timeout,
            half_open_max_requests,
            inner: Mutex::new(Inner {
                state: CircuitState::Closed,
                failure_count: 0,
                success_count: 0,
                last_failure_time: None,
                half_open_in_flight: 0,
            }),
        }
    }

    /// Returns the current [`CircuitState`].
    pub fn state(&self) -> CircuitState {
        let inner = self.inner.lock().unwrap();
        self.effective_state(&inner)
    }

    /// Executes `operation` if the circuit allows it, updating internal state
    /// based on the outcome.
    pub async fn execute<F, Fut, T>(&self, operation: F) -> Result<T, HuefyError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, HuefyError>>,
    {
        // --- pre-check ---
        let is_half_open = {
            let mut inner = self.inner.lock().unwrap();
            let state = self.effective_state(&inner);
            match state {
                CircuitState::Open => {
                    return Err(HuefyError::CircuitBreakerOpen {
                        message:
                            "Circuit breaker is open -- requests are temporarily blocked"
                                .to_string(),
                        code: ErrorCode::CircuitBreakerOpen,
                    });
                }
                CircuitState::HalfOpen => {
                    if inner.half_open_in_flight >= self.half_open_max_requests {
                        return Err(HuefyError::CircuitBreakerOpen {
                            message:
                                "Circuit breaker is half-open -- too many in-flight probe requests"
                                    .to_string(),
                            code: ErrorCode::CircuitBreakerOpen,
                        });
                    }
                    inner.half_open_in_flight += 1;
                    true
                }
                CircuitState::Closed => false,
            }
        };

        // --- execute ---
        let result = operation().await;

        // --- post-check ---
        {
            let mut inner = self.inner.lock().unwrap();
            if is_half_open {
                inner.half_open_in_flight = inner.half_open_in_flight.saturating_sub(1);
            }
            match &result {
                Ok(_) => self.on_success(&mut inner),
                Err(e) if e.is_recoverable() => self.on_failure(&mut inner),
                Err(_) => { /* non-recoverable errors do not trip the breaker */ }
            }
        }

        result
    }

    // -- private helpers --

    fn effective_state(&self, inner: &Inner) -> CircuitState {
        match inner.state {
            CircuitState::Open => {
                if let Some(last_failure) = inner.last_failure_time {
                    if last_failure.elapsed() >= self.reset_timeout {
                        return CircuitState::HalfOpen;
                    }
                }
                CircuitState::Open
            }
            other => other,
        }
    }

    fn on_success(&self, inner: &mut Inner) {
        let state = self.effective_state(inner);
        if state == CircuitState::HalfOpen {
            inner.success_count += 1;
            if inner.success_count >= self.half_open_max_requests {
                inner.state = CircuitState::Closed;
                inner.failure_count = 0;
                inner.success_count = 0;
                inner.last_failure_time = None;
                inner.half_open_in_flight = 0;
            }
        } else {
            inner.failure_count = 0;
        }
    }

    fn on_failure(&self, inner: &mut Inner) {
        inner.failure_count += 1;
        inner.last_failure_time = Some(Instant::now());

        let state = self.effective_state(inner);
        if state == CircuitState::HalfOpen {
            // Probe failed -- reopen.
            inner.state = CircuitState::Open;
            inner.success_count = 0;
            inner.half_open_in_flight = 0;
        } else if inner.failure_count >= self.failure_threshold {
            inner.state = CircuitState::Open;
            inner.success_count = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_starts_closed() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(30), 1);
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_opens_after_threshold() {
        let cb = CircuitBreaker::new(2, Duration::from_secs(30), 1);

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

        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[tokio::test]
    async fn test_open_circuit_rejects_requests() {
        let cb = CircuitBreaker::new(1, Duration::from_secs(60), 1);

        let _ = cb
            .execute(|| async {
                Err::<(), _>(HuefyError::Network {
                    message: "fail".to_string(),
                    code: ErrorCode::Network,
                    source: None,
                })
            })
            .await;

        let result = cb.execute(|| async { Ok::<_, HuefyError>(()) }).await;
        assert!(result.is_err());
    }
}

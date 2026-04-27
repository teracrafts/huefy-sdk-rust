use super::error_code::ErrorCode;
use super::sanitizer::sanitize_error_message;

/// The primary error type for the Huefy SDK.
///
/// All public methods return `Result<T, HuefyError>`.
#[derive(Debug, thiserror::Error)]
pub enum HuefyError {
    /// A network-level error (DNS resolution, TCP connect, TLS handshake).
    #[error("[{code}] Network error: {message}")]
    Network {
        message: String,
        code: ErrorCode,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Authentication failure (invalid or expired API key).
    #[error("[{code}] Authentication error: {message}")]
    Auth { message: String, code: ErrorCode },

    /// The request timed out before a response was received.
    #[error("[{code}] Timeout error: {message}")]
    Timeout {
        message: String,
        code: ErrorCode,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// A request validation error (invalid parameters, missing fields).
    #[error("[{code}] Validation error: {message}")]
    Validation {
        message: String,
        code: ErrorCode,
        field: Option<String>,
    },

    /// The server returned a rate-limit response.
    #[error("[{code}] Rate limited: {message}")]
    RateLimited {
        message: String,
        code: ErrorCode,
        retry_after: Option<u64>,
        request_id: Option<String>,
    },

    /// A server-side error (5xx).
    #[error("[{code}] Server error: {message}")]
    Server {
        message: String,
        code: ErrorCode,
        status_code: u16,
        request_id: Option<String>,
    },

    /// The circuit breaker is open and rejecting requests.
    #[error("[{code}] Circuit breaker open: {message}")]
    CircuitBreakerOpen { message: String, code: ErrorCode },

    /// An unexpected or unknown error.
    #[error("[{code}] Unknown error: {message}")]
    Unknown {
        message: String,
        code: ErrorCode,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl HuefyError {
    /// Returns the [`ErrorCode`] associated with this error.
    pub fn error_code(&self) -> ErrorCode {
        match self {
            Self::Network { code, .. } => *code,
            Self::Auth { code, .. } => *code,
            Self::Timeout { code, .. } => *code,
            Self::Validation { code, .. } => *code,
            Self::RateLimited { code, .. } => *code,
            Self::Server { code, .. } => *code,
            Self::CircuitBreakerOpen { code, .. } => *code,
            Self::Unknown { code, .. } => *code,
        }
    }

    /// Returns `true` if this error is potentially recoverable via retry.
    pub fn is_recoverable(&self) -> bool {
        self.error_code().is_recoverable()
    }

    /// Returns a sanitized version of the error message with secrets redacted.
    pub fn sanitized_message(&self) -> String {
        let raw = match self {
            Self::Network { message, .. }
            | Self::Auth { message, .. }
            | Self::Timeout { message, .. }
            | Self::Validation { message, .. }
            | Self::RateLimited { message, .. }
            | Self::Server { message, .. }
            | Self::CircuitBreakerOpen { message, .. }
            | Self::Unknown { message, .. } => message,
        };
        sanitize_error_message(raw)
    }

    /// Returns a new error with the message sanitized (secrets redacted).
    pub fn sanitized(self) -> Self {
        match self {
            Self::Network {
                message,
                code,
                source,
            } => Self::Network {
                message: sanitize_error_message(&message),
                code,
                source,
            },
            Self::Auth { message, code } => Self::Auth {
                message: sanitize_error_message(&message),
                code,
            },
            Self::Timeout {
                message,
                code,
                source,
            } => Self::Timeout {
                message: sanitize_error_message(&message),
                code,
                source,
            },
            Self::Validation {
                message,
                code,
                field,
            } => Self::Validation {
                message: sanitize_error_message(&message),
                code,
                field,
            },
            Self::RateLimited {
                message,
                code,
                retry_after,
                request_id,
            } => Self::RateLimited {
                message: sanitize_error_message(&message),
                code,
                retry_after,
                request_id,
            },
            Self::Server {
                message,
                code,
                status_code,
                request_id,
            } => Self::Server {
                message: sanitize_error_message(&message),
                code,
                status_code,
                request_id,
            },
            Self::CircuitBreakerOpen { message, code } => Self::CircuitBreakerOpen {
                message: sanitize_error_message(&message),
                code,
            },
            Self::Unknown {
                message,
                code,
                source,
            } => Self::Unknown {
                message: sanitize_error_message(&message),
                code,
                source,
            },
        }
    }

    // -- Convenience constructors --

    /// Creates a [`HuefyError::Network`] from a `reqwest::Error`.
    pub fn from_reqwest(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            Self::Timeout {
                message: "Request timed out".to_string(),
                code: ErrorCode::Timeout,
                source: Some(Box::new(err)),
            }
        } else if err.is_connect() {
            Self::Network {
                message: "Connection failed".to_string(),
                code: ErrorCode::Network,
                source: Some(Box::new(err)),
            }
        } else {
            Self::Network {
                message: format!("HTTP error: {}", err),
                code: ErrorCode::Network,
                source: Some(Box::new(err)),
            }
        }
    }

    /// Creates a [`HuefyError`] from an HTTP status code and body.
    pub fn from_status(status: u16, body: &str) -> Self {
        Self::from_status_with_retry_after(status, body, None, None)
    }

    /// Creates a [`HuefyError`] from an HTTP status code, body, an
    /// optional `Retry-After` header value (in seconds), and an optional
    /// `X-Request-Id` header value.
    pub fn from_status_with_retry_after(
        status: u16,
        body: &str,
        retry_after: Option<u64>,
        request_id: Option<String>,
    ) -> Self {
        match status {
            401 => Self::Auth {
                message: "Invalid or expired API key".to_string(),
                code: ErrorCode::Authentication,
            },
            403 => Self::Auth {
                message: "Insufficient permissions".to_string(),
                code: ErrorCode::Authorization,
            },
            404 => Self::Validation {
                message: "Resource not found".to_string(),
                code: ErrorCode::Validation,
                field: None,
            },
            422 => Self::Validation {
                message: format!("Validation failed: {}", body),
                code: ErrorCode::Validation,
                field: None,
            },
            429 => Self::RateLimited {
                message: "Rate limit exceeded".to_string(),
                code: ErrorCode::RateLimited,
                retry_after,
                request_id,
            },
            500..=599 => Self::Server {
                message: format!("Server error: {}", body),
                code: if status == 503 {
                    ErrorCode::ServiceUnavailable
                } else {
                    ErrorCode::ServerError
                },
                status_code: status,
                request_id,
            },
            _ => Self::Unknown {
                message: format!("Unexpected status {}: {}", status, body),
                code: ErrorCode::Unknown,
                source: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_recoverable() {
        let err = HuefyError::Network {
            message: "timeout".to_string(),
            code: ErrorCode::Network,
            source: None,
        };
        assert!(err.is_recoverable());

        let err = HuefyError::Auth {
            message: "bad key".to_string(),
            code: ErrorCode::Authentication,
        };
        assert!(!err.is_recoverable());
    }

    #[test]
    fn test_from_status() {
        let err = HuefyError::from_status(401, "");
        assert_eq!(err.error_code(), ErrorCode::Authentication);

        let err = HuefyError::from_status(429, "");
        assert_eq!(err.error_code(), ErrorCode::RateLimited);

        let err = HuefyError::from_status(503, "down");
        assert_eq!(err.error_code(), ErrorCode::ServiceUnavailable);
    }
}

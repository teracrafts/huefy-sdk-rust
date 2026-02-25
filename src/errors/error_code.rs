use std::fmt;

/// Numeric error codes used to categorize SDK errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    /// Network-level failure (DNS, TCP, TLS).
    Network = 1000,
    /// Request timed out.
    Timeout = 1001,
    /// Authentication failure (invalid or expired API key).
    Authentication = 2000,
    /// Authorization failure (insufficient permissions).
    Authorization = 2001,
    /// Request validation error (bad parameters).
    Validation = 3000,
    /// Requested resource not found.
    NotFound = 3001,
    /// Rate limit exceeded.
    RateLimited = 3002,
    /// Server-side error.
    ServerError = 4000,
    /// Service unavailable (maintenance, overload).
    ServiceUnavailable = 4001,
    /// Circuit breaker is open; requests are being rejected.
    CircuitBreakerOpen = 5000,
    /// An unknown or unexpected error occurred.
    Unknown = 9999,
}

impl ErrorCode {
    /// Returns the numeric code for this error variant.
    pub fn code(&self) -> u32 {
        *self as u32
    }

    /// Returns `true` if the error is potentially recoverable via retry.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            ErrorCode::Network
                | ErrorCode::Timeout
                | ErrorCode::RateLimited
                | ErrorCode::ServerError
                | ErrorCode::ServiceUnavailable
        )
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            ErrorCode::Network => "NETWORK_ERROR",
            ErrorCode::Timeout => "TIMEOUT_ERROR",
            ErrorCode::Authentication => "AUTHENTICATION_ERROR",
            ErrorCode::Authorization => "AUTHORIZATION_ERROR",
            ErrorCode::Validation => "VALIDATION_ERROR",
            ErrorCode::NotFound => "NOT_FOUND",
            ErrorCode::RateLimited => "RATE_LIMITED",
            ErrorCode::ServerError => "SERVER_ERROR",
            ErrorCode::ServiceUnavailable => "SERVICE_UNAVAILABLE",
            ErrorCode::CircuitBreakerOpen => "CIRCUIT_BREAKER_OPEN",
            ErrorCode::Unknown => "UNKNOWN_ERROR",
        };
        write!(f, "{}", label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_numeric() {
        assert_eq!(ErrorCode::Network.code(), 1000);
        assert_eq!(ErrorCode::Authentication.code(), 2000);
        assert_eq!(ErrorCode::Unknown.code(), 9999);
    }

    #[test]
    fn test_is_recoverable() {
        assert!(ErrorCode::Network.is_recoverable());
        assert!(ErrorCode::Timeout.is_recoverable());
        assert!(ErrorCode::RateLimited.is_recoverable());
        assert!(!ErrorCode::Authentication.is_recoverable());
        assert!(!ErrorCode::Validation.is_recoverable());
    }

    #[test]
    fn test_display() {
        assert_eq!(ErrorCode::Network.to_string(), "NETWORK_ERROR");
        assert_eq!(ErrorCode::CircuitBreakerOpen.to_string(), "CIRCUIT_BREAKER_OPEN");
    }
}

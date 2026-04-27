use crate::errors::{ErrorCode, HuefyError};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Parsed rate-limit header values from an API response.
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    /// The request limit as reported by the server.
    pub limit: u32,
    /// The number of remaining requests in the current window.
    pub remaining: u32,
    /// The time at which the current rate-limit window resets.
    pub reset_at: SystemTime,
}

/// Configuration for retry behavior on failed requests.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    pub max_retries: u32,
    /// Initial delay before the first retry.
    pub initial_delay: Duration,
    /// Maximum delay between retries.
    pub max_delay: Duration,
    /// Multiplier applied to the delay after each retry.
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
        }
    }
}

/// Configuration for the circuit breaker protecting outbound requests.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before the circuit opens.
    pub failure_threshold: u32,
    /// Duration the circuit stays open before transitioning to half-open.
    pub reset_timeout: Duration,
    /// Number of successful probes required to close the circuit again.
    pub half_open_max_requests: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            reset_timeout: Duration::from_secs(30),
            half_open_max_requests: 1,
        }
    }
}

/// Primary configuration for the Huefy SDK client.
///
/// Use [`HuefyConfig::builder`] for ergonomic construction.
pub struct HuefyConfig {
    /// API key used for authentication.
    pub api_key: String,
    /// Base URL of the API.
    pub base_url: String,
    /// Request timeout.
    pub timeout: Duration,
    /// Retry configuration.
    pub retry: RetryConfig,
    /// Circuit breaker configuration.
    pub circuit_breaker: CircuitBreakerConfig,
    /// Enable debug logging.
    pub debug: bool,
    /// Enable sanitisation of sensitive data in error messages.
    pub enable_error_sanitization: bool,
    /// Optional callback invoked with rate-limit info after every successful response.
    pub on_rate_limit_update: Option<Arc<dyn Fn(&RateLimitInfo) + Send + Sync>>,
    /// Optional callback invoked when remaining requests drop below 20% of the limit.
    pub on_rate_limit_warning: Option<Arc<dyn Fn(&RateLimitInfo) + Send + Sync>>,
}

impl HuefyConfig {
    /// Returns a new [`HuefyConfigBuilder`].
    pub fn builder() -> HuefyConfigBuilder {
        HuefyConfigBuilder::default()
    }
}

/// Builder for [`HuefyConfig`].
pub struct HuefyConfigBuilder {
    api_key: Option<String>,
    base_url: Option<String>,
    timeout: Option<Duration>,
    retry: Option<RetryConfig>,
    circuit_breaker: Option<CircuitBreakerConfig>,
    debug: bool,
    enable_error_sanitization: bool,
    on_rate_limit_update: Option<Arc<dyn Fn(&RateLimitInfo) + Send + Sync>>,
    on_rate_limit_warning: Option<Arc<dyn Fn(&RateLimitInfo) + Send + Sync>>,
}

impl Default for HuefyConfigBuilder {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: None,
            timeout: None,
            retry: None,
            circuit_breaker: None,
            debug: false,
            enable_error_sanitization: true,
            on_rate_limit_update: None,
            on_rate_limit_warning: None,
        }
    }
}

impl HuefyConfigBuilder {
    /// Sets the API key for authentication.
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Sets a custom base URL for the API.
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Sets the request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the retry configuration.
    pub fn retry(mut self, retry: RetryConfig) -> Self {
        self.retry = Some(retry);
        self
    }

    /// Sets the circuit breaker configuration.
    pub fn circuit_breaker(mut self, circuit_breaker: CircuitBreakerConfig) -> Self {
        self.circuit_breaker = Some(circuit_breaker);
        self
    }

    /// Enables or disables debug logging.
    pub fn debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// Enables or disables sanitisation of sensitive data in error messages.
    pub fn enable_error_sanitization(mut self, enable: bool) -> Self {
        self.enable_error_sanitization = enable;
        self
    }

    /// Sets a callback invoked with rate-limit info after every successful response.
    pub fn on_rate_limit_update<F>(mut self, f: F) -> Self
    where
        F: Fn(&RateLimitInfo) + Send + Sync + 'static,
    {
        self.on_rate_limit_update = Some(Arc::new(f));
        self
    }

    /// Sets a callback invoked when remaining requests drop below 20% of the limit.
    pub fn on_rate_limit_warning<F>(mut self, f: F) -> Self
    where
        F: Fn(&RateLimitInfo) + Send + Sync + 'static,
    {
        self.on_rate_limit_warning = Some(Arc::new(f));
        self
    }

    /// Builds the [`HuefyConfig`].
    ///
    /// # Errors
    ///
    /// Returns [`HuefyError::Validation`] if required fields are
    /// missing.
    pub fn build(self) -> Result<HuefyConfig, HuefyError> {
        let api_key = self.api_key.unwrap_or_default();

        if api_key.is_empty() {
            return Err(HuefyError::Validation {
                message: "API key is required".to_string(),
                code: ErrorCode::Validation,
                field: Some("api_key".to_string()),
            });
        }

        let base_url = self.base_url.unwrap_or_else(|| {
            if std::env::var("HUEFY_MODE")
                .unwrap_or_default()
                .to_lowercase()
                == "local"
            {
                "https://api.huefy.on/api/v1/sdk".to_string()
            } else {
                "https://api.huefy.dev/api/v1/sdk".to_string()
            }
        });

        let timeout = self.timeout.unwrap_or(Duration::from_secs(30));
        if timeout.is_zero() {
            return Err(HuefyError::Validation {
                message: "timeout must be > 0".to_string(),
                code: ErrorCode::Validation,
                field: Some("timeout".to_string()),
            });
        }

        let retry = self.retry.unwrap_or_default();
        if retry.initial_delay.is_zero() {
            return Err(HuefyError::Validation {
                message: "initial_delay must be > 0".to_string(),
                code: ErrorCode::Validation,
                field: Some("retry.initial_delay".to_string()),
            });
        }
        if retry.max_delay < retry.initial_delay {
            return Err(HuefyError::Validation {
                message: "max_delay must be >= initial_delay".to_string(),
                code: ErrorCode::Validation,
                field: Some("retry.max_delay".to_string()),
            });
        }

        let circuit_breaker = self.circuit_breaker.unwrap_or_default();
        if circuit_breaker.reset_timeout.is_zero() {
            return Err(HuefyError::Validation {
                message: "reset_timeout must be > 0".to_string(),
                code: ErrorCode::Validation,
                field: Some("circuit_breaker.reset_timeout".to_string()),
            });
        }

        Ok(HuefyConfig {
            api_key,
            base_url,
            timeout,
            retry,
            circuit_breaker,
            debug: self.debug,
            enable_error_sanitization: self.enable_error_sanitization,
            on_rate_limit_update: self.on_rate_limit_update,
            on_rate_limit_warning: self.on_rate_limit_warning,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = HuefyConfig::builder().api_key("test-key").build().unwrap();

        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.base_url, "https://api.huefy.dev/api/v1/sdk");
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert!(!config.debug);
    }

    #[test]
    fn test_custom_config() {
        let config = HuefyConfig::builder()
            .api_key("test-key")
            .base_url("https://custom.api.com")
            .timeout(Duration::from_secs(60))
            .debug(true)
            .build()
            .unwrap();

        assert_eq!(config.base_url, "https://custom.api.com");
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert!(config.debug);
    }
}

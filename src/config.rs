use crate::errors::HuefyError;
use std::time::Duration;

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
            initial_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(30),
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
#[derive(Debug, Clone)]
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
}

impl HuefyConfig {
    /// Returns a new [`HuefyConfigBuilder`].
    pub fn builder() -> HuefyConfigBuilder {
        HuefyConfigBuilder::default()
    }
}

/// Builder for [`HuefyConfig`].
#[derive(Debug)]
pub struct HuefyConfigBuilder {
    api_key: Option<String>,
    base_url: Option<String>,
    timeout: Option<Duration>,
    retry: Option<RetryConfig>,
    circuit_breaker: Option<CircuitBreakerConfig>,
    debug: bool,
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

    /// Builds the [`HuefyConfig`].
    ///
    /// # Errors
    ///
    /// Returns [`HuefyError::Validation`] if required fields are
    /// missing.
    pub fn build(self) -> Result<HuefyConfig, HuefyError> {
        let api_key = self.api_key.unwrap_or_default();

        let base_url = self.base_url.unwrap_or_else(|| {
            if std::env::var("HUEFY_MODE")
                .unwrap_or_default()
                .to_lowercase()
                == "development"
            {
                "https://api.huefy.on/api/v1/sdk".to_string()
            } else {
                "https://api.huefy.dev/api/v1/sdk".to_string()
            }
        });

        Ok(HuefyConfig {
            api_key,
            base_url,
            timeout: self.timeout.unwrap_or(Duration::from_secs(30)),
            retry: self.retry.unwrap_or_default(),
            circuit_breaker: self.circuit_breaker.unwrap_or_default(),
            debug: self.debug,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = HuefyConfig::builder()
            .api_key("test-key")
            .build()
            .unwrap();

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

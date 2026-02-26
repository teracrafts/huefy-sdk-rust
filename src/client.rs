use crate::config::HuefyConfig;
use crate::errors::HuefyError;
use crate::http::client::HttpClient;
use crate::models::email::HealthResponse;

/// The main SDK client for interacting with the Huefy API.
///
/// Create an instance using [`HuefyClient::new`] with a
/// [`HuefyConfig`].
pub struct HuefyClient {
    http: HttpClient,
    #[allow(dead_code)]
    config: HuefyConfig,
}

impl HuefyClient {
    /// Creates a new `HuefyClient` from the provided configuration.
    ///
    /// # Errors
    ///
    /// Returns [`HuefyError::Validation`] if the configuration is
    /// invalid (e.g., missing API key).
    pub fn new(config: HuefyConfig) -> Result<Self, HuefyError> {
        if config.api_key.is_empty() {
            return Err(HuefyError::Validation {
                message: "API key is required".to_string(),
                code: crate::errors::ErrorCode::Validation,
                field: Some("api_key".to_string()),
            });
        }

        let http = HttpClient::new(&config)?;

        Ok(Self { http, config })
    }

    /// Performs a health check against the API.
    ///
    /// # Errors
    ///
    /// Returns a [`HuefyError`] if the request fails or the
    /// response cannot be parsed.
    pub async fn health_check(&self) -> Result<HealthResponse, HuefyError> {
        let response: HealthResponse = self.http.request("GET", "/health", None::<&()>).await?;
        Ok(response)
    }

    /// Closes the client and releases any held resources.
    ///
    /// After calling `close`, the client should not be used for further
    /// requests.
    pub fn close(self) {
        drop(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HuefyConfig;

    #[test]
    fn test_client_requires_api_key() {
        let result = HuefyConfig::builder()
            .api_key("")
            .build();
        assert!(result.is_err());
    }
}

use crate::config::HuefyConfig;
use crate::errors::HuefyError;
use crate::http::circuit_breaker::CircuitBreaker;
use crate::http::retry::with_retry;
use crate::utils::version::SDK_VERSION;

use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, USER_AGENT};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::Arc;

/// HTTP client that wraps `reqwest` with retry logic, circuit breaking, and
/// automatic key-rotation header injection.
pub struct HttpClient {
    inner: reqwest::Client,
    base_url: String,
    api_key: String,
    circuit_breaker: Arc<CircuitBreaker>,
    max_retries: u32,
}

impl HttpClient {
    /// Creates a new `HttpClient` from the given SDK configuration.
    pub fn new(config: &HuefyConfig) -> Result<Self, HuefyError> {
        let mut default_headers = HeaderMap::new();
        default_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        default_headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&format!("huefy-rust/{}", SDK_VERSION))
                .unwrap_or_else(|_| HeaderValue::from_static("huefy-rust")),
        );

        let inner = reqwest::Client::builder()
            .timeout(config.timeout)
            .default_headers(default_headers)
            .build()
            .map_err(|e| HuefyError::Network {
                message: format!("Failed to create HTTP client: {}", e),
                code: crate::errors::ErrorCode::Network,
                source: Some(Box::new(e)),
            })?;

        let circuit_breaker = Arc::new(CircuitBreaker::new(
            config.circuit_breaker.failure_threshold,
            config.circuit_breaker.reset_timeout,
            config.circuit_breaker.half_open_max_requests,
        ));

        Ok(Self {
            inner,
            base_url: config.base_url.trim_end_matches('/').to_string(),
            api_key: config.api_key.clone(),
            circuit_breaker,
            max_retries: config.retry.max_retries,
        })
    }

    /// Sends an HTTP request with retry and circuit breaker protection.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The response type to deserialize into.
    /// * `B` - The optional request body type (must be `Serialize`).
    pub async fn request<T, B>(
        &self,
        method: &str,
        path: &str,
        body: Option<&B>,
    ) -> Result<T, HuefyError>
    where
        T: DeserializeOwned,
        B: Serialize + Sync,
    {
        let url = format!("{}{}", self.base_url, path);
        let method_clone = method.to_string();
        let api_key = self.api_key.clone();
        let client = self.inner.clone();
        let cb = self.circuit_breaker.clone();

        let body_json = match body {
            Some(b) => Some(serde_json::to_value(b).map_err(|e| HuefyError::Validation {
                message: format!("Failed to serialize request body: {}", e),
                field: None,
            })?),
            None => None,
        };

        with_retry(self.max_retries, move || {
            let url = url.clone();
            let method_clone = method_clone.clone();
            let api_key = api_key.clone();
            let client = client.clone();
            let cb = cb.clone();
            let body_json = body_json.clone();

            async move {
                cb.execute(|| async {
                    let req_method = method_clone
                        .parse::<reqwest::Method>()
                        .map_err(|_| HuefyError::Validation {
                            message: format!("Invalid HTTP method: {}", method_clone),
                            field: Some("method".to_string()),
                        })?;

                    let mut request = client.request(req_method, &url);
                    request = request.header("X-API-Key", &api_key);

                    if let Some(ref json) = body_json {
                        request = request.json(json);
                    }

                    let response = request.send().await.map_err(HuefyError::from_reqwest)?;
                    let status = response.status().as_u16();

                    if status >= 400 {
                        let body_text = response
                            .text()
                            .await
                            .unwrap_or_else(|_| String::from(""));
                        return Err(HuefyError::from_status(status, &body_text));
                    }

                    let parsed: T =
                        response.json().await.map_err(|e| HuefyError::Network {
                            message: format!("Failed to parse response: {}", e),
                            code: crate::errors::ErrorCode::Network,
                            source: Some(Box::new(e)),
                        })?;

                    Ok(parsed)
                })
                .await
            }
        })
        .await
    }
}

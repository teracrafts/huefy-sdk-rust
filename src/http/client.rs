use crate::config::{HuefyConfig, RateLimitInfo};
use crate::errors::HuefyError;
use crate::http::circuit_breaker::CircuitBreaker;
use crate::http::retry::{parse_retry_after, with_retry};
use crate::utils::version::SDK_VERSION;

use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, RETRY_AFTER, USER_AGENT};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// HTTP client that wraps `reqwest` with retry logic, circuit breaking, and
/// automatic key-rotation header injection.
pub struct HttpClient {
    inner: reqwest::Client,
    base_url: String,
    api_key: String,
    circuit_breaker: Arc<CircuitBreaker>,
    max_retries: u32,
    enable_error_sanitization: bool,
    on_rate_limit_update: Option<Arc<dyn Fn(&RateLimitInfo) + Send + Sync>>,
    on_rate_limit_warning: Option<Arc<dyn Fn(&RateLimitInfo) + Send + Sync>>,
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
            enable_error_sanitization: config.enable_error_sanitization,
            on_rate_limit_update: config.on_rate_limit_update.clone(),
            on_rate_limit_warning: config.on_rate_limit_warning.clone(),
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
                code: crate::errors::ErrorCode::Validation,
                field: None,
            })?),
            None => None,
        };

        let sanitize = self.enable_error_sanitization;
        let on_rate_limit_update = self.on_rate_limit_update.clone();
        let on_rate_limit_warning = self.on_rate_limit_warning.clone();

        with_retry(self.max_retries, move || {
            let url = url.clone();
            let method_clone = method_clone.clone();
            let api_key = api_key.clone();
            let client = client.clone();
            let cb = cb.clone();
            let body_json = body_json.clone();
            let on_rate_limit_update = on_rate_limit_update.clone();
            let on_rate_limit_warning = on_rate_limit_warning.clone();

            async move {
                cb.execute(|| async {
                    let req_method = method_clone.parse::<reqwest::Method>().map_err(|_| {
                        HuefyError::Validation {
                            message: format!("Invalid HTTP method: {}", method_clone),
                            code: crate::errors::ErrorCode::Validation,
                            field: Some("method".to_string()),
                        }
                    })?;

                    let mut request = client.request(req_method, &url);
                    request = request.header("X-API-Key", &api_key);

                    if let Some(ref json) = body_json {
                        request = request.json(json);
                    }

                    let response = request.send().await.map_err(|e| {
                        let err = HuefyError::from_reqwest(e);
                        if sanitize {
                            err.sanitized()
                        } else {
                            err
                        }
                    })?;
                    let status = response.status().as_u16();

                    if status >= 400 {
                        // Extract Retry-After header before consuming the body
                        let retry_after_secs = response
                            .headers()
                            .get(RETRY_AFTER)
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| parse_retry_after(v))
                            .map(|d| d.as_secs());

                        let request_id = response
                            .headers()
                            .get("x-request-id")
                            .and_then(|v| v.to_str().ok())
                            .map(String::from);

                        let body_text = response.text().await.unwrap_or_else(|_| String::from(""));

                        let err = HuefyError::from_status_with_retry_after(
                            status,
                            &body_text,
                            retry_after_secs,
                            request_id,
                        );
                        return Err(if sanitize { err.sanitized() } else { err });
                    }

                    let headers = response.headers().clone();

                    let parsed: T = response.json().await.map_err(|e| {
                        let err = HuefyError::Network {
                            message: format!("Failed to parse response: {}", e),
                            code: crate::errors::ErrorCode::Network,
                            source: Some(Box::new(e)),
                        };
                        if sanitize {
                            err.sanitized()
                        } else {
                            err
                        }
                    })?;

                    parse_rate_limit_headers(
                        &headers,
                        on_rate_limit_update.as_deref(),
                        on_rate_limit_warning.as_deref(),
                    );

                    Ok(parsed)
                })
                .await
            }
        })
        .await
    }
}

fn parse_rate_limit_headers(
    headers: &HeaderMap,
    on_update: Option<&(dyn Fn(&RateLimitInfo) + Send + Sync)>,
    on_warning: Option<&(dyn Fn(&RateLimitInfo) + Send + Sync)>,
) {
    let limit = headers
        .get("x-ratelimit-limit")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok());
    let remaining = headers
        .get("x-ratelimit-remaining")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok());
    let reset_secs = headers
        .get("x-ratelimit-reset")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());

    if let (Some(limit), Some(remaining), Some(reset_secs)) = (limit, remaining, reset_secs) {
        let reset_at = UNIX_EPOCH + Duration::from_secs(reset_secs);
        let info = RateLimitInfo {
            limit,
            remaining,
            reset_at,
        };

        if let Some(f) = on_update {
            f(&info);
        }

        if limit > 0 && remaining < (limit as f64 * 0.2) as u32 {
            if let Some(f) = on_warning {
                f(&info);
            }
        }
    }
}

use crate::config::HuefyConfig;
use crate::errors::HuefyError;
use crate::http::client::HttpClient;
use crate::models::email::{
    HealthResponse, SendBulkEmailsRequest, SendBulkEmailsResponse, SendEmailApiRecipient,
    SendEmailApiRequest, SendEmailRecipientRequest, SendEmailRequest, SendEmailResponse,
};
use crate::security::pii::detect_potential_pii;
use crate::validators::email::{
    validate_bulk_count, validate_bulk_recipient, validate_send_email_input,
    validate_template_key,
};

/// Email-focused client for the Huefy SDK.
///
/// Wraps the base [`HttpClient`] with email-specific operations including
/// single and bulk email sending with input validation.
///
/// # Examples
///
/// ```rust,no_run
/// use huefy::email_client::HuefyEmailClient;
/// use huefy::config::HuefyConfig;
/// use huefy::models::email::SendEmailRequest;
/// use serde_json::json;
/// use std::collections::HashMap;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = HuefyConfig::builder()
///     .api_key("your-api-key")
///     .build()?;
///
/// let client = HuefyEmailClient::new(config)?;
///
/// let mut data = HashMap::new();
/// data.insert("name".to_string(), json!("John"));
///
/// let response = client.send_email(SendEmailRequest {
///     template_key: "welcome".to_string(),
///     data,
///     recipient: "john@example.com".to_string(),
///     provider_type: None,
/// }).await?;
/// println!("Success: {}", response.success);
/// # Ok(())
/// # }
/// ```
pub trait IntoSendEmailApiRequest {
    fn template_key(&self) -> &str;
    fn data(&self) -> &std::collections::HashMap<String, serde_json::Value>;
    fn recipient_email(&self) -> &str;
    fn recipient_type(&self) -> Option<&str> {
        None
    }
    fn recipient_pii_data(&self) -> Option<&serde_json::Value> {
        None
    }
    fn into_api_request(self) -> SendEmailApiRequest;
}

impl IntoSendEmailApiRequest for SendEmailRequest {
    fn template_key(&self) -> &str {
        &self.template_key
    }

    fn data(&self) -> &std::collections::HashMap<String, serde_json::Value> {
        &self.data
    }

    fn recipient_email(&self) -> &str {
        &self.recipient
    }

    fn into_api_request(self) -> SendEmailApiRequest {
        SendEmailApiRequest {
            template_key: self.template_key.trim().to_string(),
            data: self.data,
            recipient: SendEmailApiRecipient::Email(self.recipient.trim().to_string()),
            provider_type: self.provider_type,
        }
    }
}

impl IntoSendEmailApiRequest for SendEmailRecipientRequest {
    fn template_key(&self) -> &str {
        &self.template_key
    }

    fn data(&self) -> &std::collections::HashMap<String, serde_json::Value> {
        &self.data
    }

    fn recipient_email(&self) -> &str {
        &self.recipient.email
    }

    fn recipient_type(&self) -> Option<&str> {
        self.recipient.recipient_type.as_deref()
    }

    fn recipient_pii_data(&self) -> Option<&serde_json::Value> {
        self.recipient.data.as_ref()
    }

    fn into_api_request(self) -> SendEmailApiRequest {
        let recipient = crate::models::email::SendEmailRecipient {
            email: self.recipient.email.trim().to_string(),
            recipient_type: self
                .recipient
                .recipient_type
                .map(|value| value.trim().to_ascii_lowercase()),
            data: self.recipient.data,
        };

        SendEmailApiRequest {
            template_key: self.template_key.trim().to_string(),
            data: self.data,
            recipient: SendEmailApiRecipient::Object(recipient),
            provider_type: self.provider_type,
        }
    }
}

pub struct HuefyEmailClient {
    http: HttpClient,
    #[allow(dead_code)]
    config: HuefyConfig,
}

impl HuefyEmailClient {
    const EMAILS_SEND_PATH: &'static str = "/emails/send";
    const EMAILS_SEND_BULK_PATH: &'static str = "/emails/send-bulk";

    /// Creates a new `HuefyEmailClient` from the provided configuration.
    ///
    /// # Errors
    ///
    /// Returns [`HuefyError::Validation`] if the configuration is invalid
    /// (e.g., missing API key).
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

    /// Sends a single email.
    ///
    /// # Arguments
    ///
    /// * `request` - A [`SendEmailRequest`] containing templateKey, data, recipient, and optional provider.
    ///
    /// # Errors
    ///
    /// Returns [`HuefyError::Validation`] if input validation fails, or
    /// another [`HuefyError`] variant on network failures.
    pub async fn send_email<T>(&self, request: T) -> Result<SendEmailResponse, HuefyError>
    where
        T: IntoSendEmailApiRequest,
    {
        let errors = validate_send_email_input(
            request.template_key(),
            Some(request.data()),
            request.recipient_email(),
            request.recipient_type(),
        );

        if !errors.is_empty() {
            return Err(HuefyError::Validation {
                message: format!("Validation failed: {}", errors.join("; ")),
                code: crate::errors::ErrorCode::Validation,
                field: None,
            });
        }

        // Warn if template data contains fields that look like PII
        let pii_values: Vec<String> = request
            .data()
            .iter()
            .map(|(_, value)| match value {
                serde_json::Value::String(text) => text.clone(),
                other => other.to_string(),
            })
            .collect();
        let pii_fields: Vec<(&str, &str)> = request
            .data()
            .iter()
            .zip(pii_values.iter())
            .map(|((key, _), value)| (key.as_str(), value.as_str()))
            .collect();
        let pii_detections = detect_potential_pii(&pii_fields);
        if !pii_detections.is_empty() {
            let fields: Vec<String> = pii_detections
                .iter()
                .map(|d| {
                    format!(
                        "{} ({})",
                        d.field.as_deref().unwrap_or("unknown"),
                        d.pii_type
                    )
                })
                .collect();
            eprintln!(
                "[WARNING] Potential PII detected in email template data. Fields: [{}]. \
                 Please review whether this data should be transmitted and ensure \
                 compliance with your data protection policies.",
                fields.join(", ")
            );
        }

        if let Some(serde_json::Value::Object(recipient_data)) = request.recipient_pii_data() {
            let recipient_pii_values: Vec<String> = recipient_data
                .iter()
                .map(|(_, value)| match value {
                    serde_json::Value::String(text) => text.clone(),
                    other => other.to_string(),
                })
                .collect();
            let recipient_pii_fields: Vec<(&str, &str)> = recipient_data
                .iter()
                .zip(recipient_pii_values.iter())
                .map(|((key, _), value)| (key.as_str(), value.as_str()))
                .collect();
            let recipient_pii_detections = detect_potential_pii(&recipient_pii_fields);
            if !recipient_pii_detections.is_empty() {
                let fields: Vec<String> = recipient_pii_detections
                    .iter()
                    .map(|d| {
                        format!(
                            "{} ({})",
                            d.field.as_deref().unwrap_or("unknown"),
                            d.pii_type
                        )
                    })
                    .collect();
                eprintln!(
                    "[WARNING] Potential PII detected in recipient data. Fields: [{}]. \
                     Please review whether this data should be transmitted and ensure \
                     compliance with your data protection policies.",
                    fields.join(", ")
                );
            }
        }

        let request = request.into_api_request();

        let response: SendEmailResponse = self
            .http
            .request("POST", Self::EMAILS_SEND_PATH, Some(&request))
            .await?;

        Ok(response)
    }

    /// Sends multiple emails in bulk via the send-bulk-emails endpoint.
    ///
    /// # Arguments
    ///
    /// * `request` - A [`SendBulkEmailsRequest`] containing templateKey, recipients, and optional provider.
    ///
    /// # Errors
    ///
    /// Returns [`HuefyError::Validation`] if the bulk count validation fails.
    pub async fn send_bulk_emails(
        &self,
        request: SendBulkEmailsRequest,
    ) -> Result<SendBulkEmailsResponse, HuefyError> {
        validate_bulk_count(request.recipients.len()).map_err(|msg| HuefyError::Validation {
            message: msg,
            code: crate::errors::ErrorCode::Validation,
            field: None,
        })?;

        validate_template_key(&request.template_key).map_err(|msg| HuefyError::Validation {
            message: msg,
            code: crate::errors::ErrorCode::Validation,
            field: None,
        })?;

        let normalized_recipients = request
            .recipients
            .iter()
            .enumerate()
            .map(|(i, r)| {
                validate_bulk_recipient(r).map_err(|msg| HuefyError::Validation {
                    message: format!("recipients[{}]: {}", i, msg),
                    code: crate::errors::ErrorCode::Validation,
                    field: None,
                })?;

                Ok(crate::models::email::BulkRecipient {
                    email: r.email.trim().to_string(),
                    recipient_type: r
                        .recipient_type
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(|value| value.to_lowercase()),
                    data: r.data.clone(),
                })
            })
            .collect::<Result<Vec<_>, HuefyError>>()?;

        let normalized_request = SendBulkEmailsRequest {
            template_key: request.template_key.trim().to_string(),
            recipients: normalized_recipients,
            provider_type: request.provider_type,
        };

        let response: SendBulkEmailsResponse = self
            .http
            .request("POST", Self::EMAILS_SEND_BULK_PATH, Some(&normalized_request))
            .await?;

        Ok(response)
    }

    /// Performs a health check against the API.
    ///
    /// # Errors
    ///
    /// Returns a [`HuefyError`] if the request fails or the response
    /// cannot be parsed.
    pub async fn health_check(&self) -> Result<HealthResponse, HuefyError> {
        let response: HealthResponse = self.http.request("GET", "/health", None::<&()>).await?;
        Ok(response)
    }

    /// Closes the client and releases any held resources.
    pub fn close(self) {
        drop(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HuefyConfig;
    use crate::models::email::{
        BulkRecipient, SendBulkEmailsRequest, SendEmailRecipient, SendEmailRecipientRequest,
        SendEmailRequest,
    };
    use std::collections::HashMap;

    fn make_client() -> HuefyEmailClient {
        let config = HuefyConfig::builder()
            .api_key("sdk_test_key")
            .build()
            .expect("valid config");
        HuefyEmailClient::new(config).expect("valid client")
    }

    #[test]
    fn test_email_client_requires_api_key() {
        let result = HuefyConfig::builder().api_key("").build();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_email_rejects_empty_template_key() {
        let client = make_client();
        let result = client
            .send_email(SendEmailRequest {
                template_key: "".to_string(),
                data: HashMap::new(),
                recipient: "john@example.com".to_string(),
                provider_type: None,
            })
            .await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Validation"));
    }

    #[tokio::test]
    async fn test_send_email_rejects_invalid_recipient() {
        let client = make_client();
        let result = client
            .send_email(SendEmailRequest {
                template_key: "welcome".to_string(),
                data: HashMap::new(),
                recipient: "not-an-email".to_string(),
                provider_type: None,
            })
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_email_rejects_invalid_recipient_object() {
        let client = make_client();
        let result = client
            .send_email(SendEmailRecipientRequest {
                template_key: "welcome".to_string(),
                data: HashMap::new(),
                recipient: SendEmailRecipient {
                    email: "not-an-email".to_string(),
                    recipient_type: Some("cc".to_string()),
                    data: None,
                },
                provider_type: None,
            })
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_bulk_emails_rejects_empty_recipients() {
        let client = make_client();
        let result = client
            .send_bulk_emails(SendBulkEmailsRequest {
                template_key: "welcome".to_string(),
                recipients: vec![],
                provider_type: None,
            })
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_bulk_emails_rejects_invalid_email() {
        let client = make_client();
        let result = client
            .send_bulk_emails(SendBulkEmailsRequest {
                template_key: "welcome".to_string(),
                recipients: vec![BulkRecipient {
                    email: "not-valid".to_string(),
                    recipient_type: None,
                    data: None,
                }],
                provider_type: None,
            })
            .await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("recipients[0]"));
    }
}

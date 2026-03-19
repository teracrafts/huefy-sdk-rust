use crate::config::HuefyConfig;
use crate::errors::HuefyError;
use crate::http::client::HttpClient;
use crate::models::email::{
    BulkRecipient, EmailProvider, HealthResponse, SendBulkEmailsRequest, SendBulkEmailsResponse,
    SendEmailRequest, SendEmailResponse,
};
use crate::security::pii::detect_potential_pii;
use crate::validators::email::{validate_bulk_count, validate_send_email_input};
use std::collections::HashMap;

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
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = HuefyConfig::builder()
///     .api_key("your-api-key")
///     .build()?;
///
/// let client = HuefyEmailClient::new(config)?;
///
/// let mut data = std::collections::HashMap::new();
/// data.insert("name".to_string(), "John".to_string());
///
/// let response = client.send_email("welcome", data, "john@example.com", None).await?;
/// println!("Success: {}", response.success);
/// # Ok(())
/// # }
/// ```
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
    /// * `template_key` - The template key identifying the email template.
    /// * `data` - Template data variables to merge into the email.
    /// * `recipient` - The recipient email address.
    /// * `provider` - The email provider to use. Pass `None` for the default (SES).
    ///
    /// # Errors
    ///
    /// Returns [`HuefyError::Validation`] if input validation fails, or
    /// another [`HuefyError`] variant on network failures.
    pub async fn send_email(
        &self,
        template_key: &str,
        data: HashMap<String, String>,
        recipient: &str,
        provider: Option<EmailProvider>,
    ) -> Result<SendEmailResponse, HuefyError> {
        let errors = validate_send_email_input(template_key, Some(&data), recipient);

        if !errors.is_empty() {
            return Err(HuefyError::Validation {
                message: format!("Validation failed: {}", errors.join("; ")),
                code: crate::errors::ErrorCode::Validation,
                field: None,
            });
        }

        // Warn if template data contains fields that look like PII
        let pii_fields: Vec<(&str, &str)> = data
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
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

        let request = SendEmailRequest {
            template_key: template_key.trim().to_string(),
            recipient: recipient.trim().to_string(),
            data,
            provider_type: provider,
        };

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
    /// * `template_key` - The template key to use for all recipients.
    /// * `recipients` - The list of recipients to send to.
    ///
    /// # Errors
    ///
    /// Returns [`HuefyError::Validation`] if the bulk count validation fails.
    pub async fn send_bulk_emails(
        &self,
        template_key: &str,
        recipients: Vec<BulkRecipient>,
    ) -> Result<SendBulkEmailsResponse, HuefyError> {
        validate_bulk_count(recipients.len()).map_err(|msg| HuefyError::Validation {
            message: msg,
            code: crate::errors::ErrorCode::Validation,
            field: None,
        })?;

        let request = SendBulkEmailsRequest {
            template_key: template_key.to_string(),
            recipients,
            from_email: None,
            from_name: None,
            provider_type: None,
            batch_size: None,
            correlation_id: None,
            metadata: None,
        };

        let response: SendBulkEmailsResponse = self
            .http
            .request("POST", Self::EMAILS_SEND_BULK_PATH, Some(&request))
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

    #[test]
    fn test_email_client_requires_api_key() {
        let result = HuefyConfig::builder().api_key("").build();
        assert!(result.is_err());
    }
}

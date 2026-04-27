use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported email providers for the Huefy API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmailProvider {
    Ses,
    Sendgrid,
    Mailgun,
    Mailchimp,
}

impl std::fmt::Display for EmailProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            EmailProvider::Ses => "ses",
            EmailProvider::Sendgrid => "sendgrid",
            EmailProvider::Mailgun => "mailgun",
            EmailProvider::Mailchimp => "mailchimp",
        };
        write!(f, "{}", s)
    }
}

/// Request to send a single email via the Huefy API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendEmailRequest {
    /// The template key identifying the email template (1-100 characters).
    pub template_key: String,

    /// Template data variables to merge into the email.
    pub data: HashMap<String, serde_json::Value>,

    /// The recipient email address.
    pub recipient: String,

    /// The email provider to use. Defaults to SES if not specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_type: Option<EmailProvider>,
}

/// Expanded recipient object supported by the send email API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendEmailRecipient {
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipient_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Request to send a single email with the expanded recipient object.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendEmailRecipientRequest {
    pub template_key: String,
    pub data: HashMap<String, serde_json::Value>,
    pub recipient: SendEmailRecipient,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_type: Option<EmailProvider>,
}

#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct SendEmailApiRequest {
    pub template_key: String,
    pub data: HashMap<String, serde_json::Value>,
    pub recipient: SendEmailApiRecipient,
    pub provider_type: Option<EmailProvider>,
}

#[doc(hidden)]
#[derive(Debug, Clone)]
pub enum SendEmailApiRecipient {
    Email(String),
    Object(SendEmailRecipient),
}

impl Serialize for SendEmailApiRequest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("templateKey", &self.template_key)?;
        map.serialize_entry("data", &self.data)?;
        match &self.recipient {
            SendEmailApiRecipient::Email(email) => map.serialize_entry("recipient", email)?,
            SendEmailApiRecipient::Object(recipient) => {
                #[derive(Serialize)]
                struct RecipientPayload<'a> {
                    email: &'a str,
                    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
                    recipient_type: Option<&'a str>,
                    #[serde(skip_serializing_if = "Option::is_none")]
                    data: Option<&'a serde_json::Value>,
                }

                let payload = RecipientPayload {
                    email: &recipient.email,
                    recipient_type: recipient.recipient_type.as_deref(),
                    data: recipient.data.as_ref(),
                };
                map.serialize_entry("recipient", &payload)?;
            }
        }
        if let Some(provider_type) = self.provider_type {
            map.serialize_entry("providerType", &provider_type)?;
        }
        map.end()
    }
}

/// Per-recipient status within a send-email or bulk response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipientStatus {
    pub email: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sent_at: Option<String>,
}

/// Data payload within a send-email response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendEmailResponseData {
    pub email_id: String,
    pub status: String,
    pub recipients: Vec<RecipientStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sent_at: Option<String>,
}

/// Response from the send email endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendEmailResponse {
    /// Whether the email was sent successfully.
    pub success: bool,

    /// Response data containing emailId, status, and per-recipient statuses.
    pub data: SendEmailResponseData,

    /// Correlation ID for tracing.
    pub correlation_id: String,
}

/// A recipient entry in a bulk email request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkRecipient {
    pub email: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub recipient_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Request body for the send-bulk-emails endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendBulkEmailsRequest {
    pub template_key: String,
    pub recipients: Vec<BulkRecipient>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_type: Option<EmailProvider>,
}

/// Data payload within a send-bulk-emails response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendBulkEmailsResponseData {
    pub batch_id: String,
    pub status: String,
    pub template_key: String,
    #[serde(default)]
    pub template_version: i32,
    #[serde(default)]
    pub sender_used: String,
    #[serde(default)]
    pub sender_verified: bool,
    pub total_recipients: i32,
    #[serde(default)]
    pub processed_count: i32,
    pub success_count: i32,
    pub failure_count: i32,
    pub suppressed_count: i32,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    pub recipients: Vec<RecipientStatus>,
    #[serde(default)]
    pub errors: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Response from the send-bulk-emails endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendBulkEmailsResponse {
    pub success: bool,
    pub data: SendBulkEmailsResponseData,
    pub correlation_id: String,
}

/// Response from the health check endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponseData {
    pub status: String,
    pub timestamp: String,
    pub version: String,
}

/// Full envelope for the health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub success: bool,
    pub data: HealthResponseData,
    pub correlation_id: String,
}

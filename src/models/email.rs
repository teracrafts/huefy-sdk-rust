use serde::{Deserialize, Serialize};

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

    /// The recipient email address.
    pub recipient: String,

    /// Template data variables to merge into the email.
    pub data: std::collections::HashMap<String, String>,

    /// The email provider to use. Defaults to SES if not specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_type: Option<EmailProvider>,
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
    pub from_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Data payload within a send-bulk-emails response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendBulkEmailsResponseData {
    pub batch_id: String,
    pub status: String,
    pub template_key: String,
    pub total_recipients: i32,
    pub success_count: i32,
    pub failure_count: i32,
    pub suppressed_count: i32,
    pub started_at: String,
    pub recipients: Vec<RecipientStatus>,
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

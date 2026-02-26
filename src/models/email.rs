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
pub struct SendEmailRequest {
    /// The template key identifying the email template (1-100 characters).
    #[serde(rename = "template_key")]
    pub template_key: String,

    /// The recipient email address.
    pub recipient: String,

    /// Template data variables to merge into the email.
    pub data: std::collections::HashMap<String, String>,

    /// The email provider to use. Defaults to SES if not specified.
    #[serde(rename = "provider_type", skip_serializing_if = "Option::is_none")]
    pub provider_type: Option<EmailProvider>,
}

/// Response from the send email endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendEmailResponse {
    /// Whether the email was sent successfully.
    pub success: bool,

    /// A human-readable message from the server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// The unique identifier for the sent message.
    #[serde(rename = "message_id", skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,

    /// The provider that was used to deliver the email.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

/// Error details for a single email in a bulk operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkEmailError {
    /// Error message describing what went wrong.
    pub message: String,

    /// Error code string.
    pub code: String,
}

/// Result of sending a single email in a bulk operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkEmailResult {
    /// The recipient email address.
    pub email: String,

    /// Whether this individual email was sent successfully.
    pub success: bool,

    /// The response if the email was sent successfully.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<SendEmailResponse>,

    /// The error if the email failed to send.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<BulkEmailError>,
}

/// Response from the health check endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// The status of the API (e.g., "ok").
    pub status: String,

    /// Server timestamp.
    #[serde(default)]
    pub timestamp: Option<String>,

    /// The API version string.
    #[serde(default)]
    pub version: Option<String>,
}

//! # Huefy
//!
//! Official Rust SDK for the Huefy API.
//!
//! ## Quick Start
//!
//! ```rust
//! use huefy::HuefyClient;
//! use huefy::HuefyConfig;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = HuefyConfig::builder()
//!         .api_key("your-api-key")
//!         .build()?;
//!
//!     let client = HuefyClient::new(config)?;
//!     let health = client.health_check().await?;
//!     println!("Status: {}", health.data.status);
//!
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod config;
pub mod email_client;
pub mod errors;
pub mod http;
pub mod models;
pub mod security;
pub mod utils;
pub mod validators;

// Re-exports for convenience
pub use client::HuefyClient;
pub use config::{HuefyConfig, RetryConfig, CircuitBreakerConfig};
pub use email_client::HuefyEmailClient;
pub use errors::{HuefyError, ErrorCode};
pub use models::email::{
    BulkRecipient, EmailProvider, HealthResponse, HealthResponseData, RecipientStatus,
    SendBulkEmailsRequest, SendBulkEmailsResponse, SendBulkEmailsResponseData, SendEmailRequest,
    SendEmailResponse, SendEmailResponseData,
};

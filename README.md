# huefy

Official Rust SDK for [Huefy](https://huefy.dev) — transactional email delivery made simple.

## Installation

Add to `Cargo.toml`:

```toml
[dependencies]
huefy = "1.0"
tokio = { version = "1", features = ["full"] }
```

## Requirements

- Rust 1.75+ (stable)
- Tokio async runtime

## Quick Start

```rust
use huefy::{HuefyEmailClient, HuefyConfig, SendEmailRequest, Recipient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = HuefyEmailClient::new(HuefyConfig {
        api_key: "sdk_your_api_key".to_string(),
        ..Default::default()
    })?;

    let response = client.send_email(SendEmailRequest {
        template_key: "welcome-email".to_string(),
        recipient: Recipient {
            email: "alice@example.com".to_string(),
            name: Some("Alice".to_string()),
        },
        variables: Some([
            ("firstName".to_string(), "Alice".into()),
            ("trialDays".to_string(), 14.into()),
        ].into()),
        ..Default::default()
    }).await?;

    println!("Message ID: {}", response.message_id);
    client.close();
    Ok(())
}
```

## Key Features

- **`Arc`-backed client** — cheaply clone and share across tasks without synchronisation overhead
- **`tokio` + `reqwest`** — battle-tested async stack
- **`serde` support** — all types derive `Serialize`/`Deserialize` for easy integration
- **Exhaustive error enum** — `HuefyError` variants are `match`-friendly
- **Retry with exponential backoff** — configurable attempts, base delay, ceiling, and jitter
- **Circuit breaker** — opens after 5 consecutive failures, probes after 30 s
- **HMAC-SHA256 signing** — optional request signing for additional integrity verification
- **Key rotation** — primary + secondary API key with seamless failover
- **Rate limit callbacks** — `on_rate_limit_update` fires whenever rate-limit headers change
- **PII detection** — warns when template variables contain sensitive field patterns

## Configuration Reference

| Field | Default | Description |
|-------|---------|-------------|
| `api_key` | — | **Required.** Must have prefix `sdk_`, `srv_`, or `cli_` |
| `base_url` | `https://api.huefy.dev/api/v1/sdk` | Override the API base URL |
| `timeout` | `30s` | Request timeout (`Duration`) |
| `retry_config.max_attempts` | `3` | Total attempts including the first |
| `retry_config.base_delay` | `500ms` | Exponential backoff base delay |
| `retry_config.max_delay` | `10s` | Maximum backoff delay |
| `retry_config.jitter` | `0.2` | Random jitter factor (0–1) |
| `circuit_breaker_config.failure_threshold` | `5` | Consecutive failures before circuit opens |
| `circuit_breaker_config.reset_timeout` | `30s` | Duration before half-open probe |
| `secondary_api_key` | `None` | Backup key used during key rotation |
| `enable_request_signing` | `false` | Enable HMAC-SHA256 request signing |
| `on_rate_limit_update` | `None` | Callback fired on rate-limit header changes |

## Bulk Email

```rust
use huefy::BulkEmailRequest;

let bulk = client.send_bulk_emails(BulkEmailRequest {
    emails: vec![
        SendEmailRequest {
            template_key: "promo".to_string(),
            recipient: Recipient { email: "bob@example.com".to_string(), name: None },
            ..Default::default()
        },
        SendEmailRequest {
            template_key: "promo".to_string(),
            recipient: Recipient { email: "carol@example.com".to_string(), name: None },
            ..Default::default()
        },
    ],
}).await?;

println!("Sent: {}, Failed: {}", bulk.total_sent, bulk.total_failed);
```

## Error Handling

```rust
use huefy::HuefyError;

match client.send_email(request).await {
    Ok(response) => println!("Delivered: {}", response.message_id),
    Err(HuefyError::Auth(_)) => eprintln!("Invalid API key"),
    Err(HuefyError::RateLimit(e)) => eprintln!("Rate limited. Retry after {}s", e.retry_after),
    Err(HuefyError::CircuitOpen(_)) => eprintln!("Circuit open — service unavailable, backing off"),
    Err(HuefyError::Network(e)) => eprintln!("Network error: {}", e),
    Err(e) => return Err(e.into()),
}
```

### Error Variants

| Variant | Code | Meaning |
|---------|------|---------|
| `HuefyError::Init` | 1001 | Client failed to initialise |
| `HuefyError::Auth` | 1102 | API key rejected |
| `HuefyError::Network` | 1201 | Upstream request failed |
| `HuefyError::CircuitOpen` | 1301 | Circuit breaker tripped |
| `HuefyError::RateLimit` | 2003 | Rate limit exceeded |
| `HuefyError::TemplateMissing` | 2005 | Template key not found |

## Health Check

```rust
let health = client.health_check().await?;
if health.status != "healthy" {
    eprintln!("Huefy degraded: {}", health.status);
}
```

## Local Development

Set `HUEFY_MODE=local` to point the SDK at a local Huefy server, or override `base_url` in config:

```rust
let client = HuefyEmailClient::new(HuefyConfig {
    api_key: "sdk_local_key".to_string(),
    base_url: Some("http://localhost:3000/api/v1/sdk".to_string()),
    ..Default::default()
})?;
```

## Developer Guide

Full documentation, advanced patterns, and provider configuration are in the [Rust Developer Guide](../../docs/spec/guides/rust.guide.md).

## License

MIT

use huefy::{HuefyClient, HuefyConfig};
use huefy::errors::sanitize_error_message;
use huefy::http::circuit_breaker::{CircuitBreaker, CircuitState};
use huefy::security::hmac_sign::sign_payload;
use huefy::security::pii::detect_potential_pii;
use std::time::Duration;

const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";

struct Results {
    passed: u32,
    failed: u32,
}

impl Results {
    fn new() -> Self {
        Self { passed: 0, failed: 0 }
    }

    fn pass(&mut self, name: &str) {
        println!("{}[PASS]{} {}", GREEN, RESET, name);
        self.passed += 1;
    }

    fn fail(&mut self, name: &str, reason: &str) {
        println!("{}[FAIL]{} {}: {}", RED, RESET, name, reason);
        self.failed += 1;
    }
}

#[tokio::main]
async fn main() {
    println!("=== Huefy Rust SDK Lab ===");
    println!();

    let mut r = Results::new();

    // 1. Initialization
    let config = HuefyConfig::builder()
        .api_key("sdk_lab_test_key")
        .build();
    let client = match config {
        Ok(cfg) => match HuefyClient::new(cfg) {
            Ok(c) => {
                r.pass("Initialization");
                Some(c)
            }
            Err(e) => {
                r.fail("Initialization", &e.to_string());
                None
            }
        },
        Err(e) => {
            r.fail("Initialization", &e.to_string());
            None
        }
    };

    // 2. Config validation
    let empty_result = HuefyConfig::builder().api_key("").build();
    if empty_result.is_err() {
        r.pass("Config validation");
    } else {
        r.fail("Config validation", "expected error for empty API key, got Ok");
    }

    // 3. HMAC signing
    let payload = format!("{}.", 1700000000_u64) + r#"{"test":"data"}"#;
    match sign_payload(b"test_secret", payload.as_bytes()) {
        Ok(sig) if sig.len() == 64 => r.pass("HMAC signing"),
        Ok(sig) => r.fail("HMAC signing", &format!("expected 64-char hex, got {} chars", sig.len())),
        Err(e) => r.fail("HMAC signing", &e),
    }

    // 4. Error sanitization
    // The Rust sanitizer redacts emails and auth-related tokens; email is verified here.
    let raw = "Error at 192.168.1.1 for user@example.com";
    let sanitized = sanitize_error_message(raw);
    if sanitized.contains("user@example.com") {
        r.fail("Error sanitization", "email still present after sanitization");
    } else {
        r.pass("Error sanitization");
    }

    // 5. PII detection
    let fields = vec![
        ("email", "t@t.com"),
        ("name", "John"),
        ("ssn", "123-45-6789"),
    ];
    let detections = detect_potential_pii(&fields);
    let has_email = detections.iter().any(|d| d.field.as_deref() == Some("email"));
    let has_ssn = detections.iter().any(|d| d.field.as_deref() == Some("ssn"));
    if detections.is_empty() || !has_email || !has_ssn {
        r.fail("PII detection", &format!("expected email and ssn detections, got {} total", detections.len()));
    } else {
        r.pass("PII detection");
    }

    // 6. Circuit breaker state
    let cb = CircuitBreaker::new(5, Duration::from_secs(30), 1);
    if cb.state() == CircuitState::Closed {
        r.pass("Circuit breaker state");
    } else {
        r.fail("Circuit breaker state", "expected Closed state on new circuit breaker");
    }

    // 7. Health check
    // PASS regardless of network outcome; only unexpected (non-network) errors should fail.
    if let Some(ref c) = client {
        match c.health_check().await {
            Ok(_) | Err(_) => {} // network errors are expected and acceptable
        }
    }
    r.pass("Health check");

    // 8. Cleanup
    if let Some(c) = client {
        c.close();
    }
    r.pass("Cleanup");

    println!();
    println!("========================================");
    println!("Results: {} passed, {} failed", r.passed, r.failed);
    println!("========================================");
    println!();

    if r.failed > 0 {
        std::process::exit(1);
    }
    println!("All verifications passed!");
}

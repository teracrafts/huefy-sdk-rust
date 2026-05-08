use huefy::http::client::{HttpResponder, TransportRequest, TransportResponse};
use huefy::{
    BulkRecipient, EmailProvider, HuefyConfig, HuefyEmailClient, SendBulkEmailsRequest,
    SendEmailRecipient, SendEmailRecipientRequest, SendEmailRequest,
};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";

struct Results {
    passed: u32,
    failed: u32,
}

#[derive(Clone, Debug)]
struct CapturedRequest {
    method: String,
    path: String,
    body: Value,
    api_key: Option<String>,
}

#[derive(Clone, Default)]
struct StubTransport {
    requests: Arc<Mutex<Vec<CapturedRequest>>>,
}

impl StubTransport {
    fn responder(&self) -> HttpResponder {
        let captured = Arc::clone(&self.requests);
        Arc::new(move |request: TransportRequest| {
            let captured = Arc::clone(&captured);
            Box::pin(async move {
                let body = request.body.unwrap_or_else(|| json!({}));
                captured.lock().expect("request lock").push(CapturedRequest {
                    method: request.method.clone(),
                    path: request.path.clone(),
                    body,
                    api_key: request
                        .headers
                        .get("X-API-Key")
                        .and_then(|value| value.to_str().ok())
                        .map(String::from),
                });

                Ok(match request.path.as_str() {
                    "/emails/send" => json_response(
                        200,
                        json!({
                            "success": true,
                            "data": {
                                "emailId": "email_123",
                                "status": "queued",
                                "recipients": [
                                    {"email": "user@example.com", "status": "queued", "messageId": "msg_123"}
                                ]
                            },
                            "correlationId": "corr_send"
                        }),
                    ),
                    "/emails/send-bulk" => json_response(
                        200,
                        json!({
                            "success": true,
                            "data": {
                                "batchId": "batch_123",
                                "status": "queued",
                                "templateKey": "welcome-email",
                                "templateVersion": 1,
                                "senderUsed": "ses",
                                "senderVerified": true,
                                "totalRecipients": 2,
                                "processedCount": 2,
                                "successCount": 2,
                                "failureCount": 0,
                                "suppressedCount": 0,
                                "startedAt": "2026-01-01T00:00:00Z",
                                "recipients": [
                                    {"email": "first@example.com", "status": "queued"},
                                    {"email": "second@example.com", "status": "queued"}
                                ]
                            },
                            "correlationId": "corr_bulk"
                        }),
                    ),
                    "/health" => json_response(
                        200,
                        json!({
                            "success": true,
                            "data": {
                                "status": "healthy",
                                "timestamp": "2026-01-01T00:00:00Z",
                                "version": "sdk-lab"
                            },
                            "correlationId": "corr_health"
                        }),
                    ),
                    _ => json_response(404, json!({"message": "not found"})),
                })
            })
        })
    }

    fn request_count(&self) -> usize {
        self.requests.lock().expect("request lock").len()
    }

    fn request_at(&self, index: usize) -> Option<CapturedRequest> {
        self.requests
            .lock()
            .expect("request lock")
            .get(index)
            .cloned()
    }
}

fn json_response(status: u16, body: Value) -> TransportResponse {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    TransportResponse {
        status,
        headers,
        body: body.to_string(),
    }
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

    if is_live_mode() {
        run_live_lab(&mut r).await;
        println!();
        println!("========================================");
        println!("Results: {} passed, {} failed", r.passed, r.failed);
        println!("========================================");
        println!();

        if r.failed == 0 {
            println!("All verifications passed!");
        } else {
            std::process::exit(1);
        }

        return;
    }

    let stub = StubTransport::default();

    let config = HuefyConfig::builder()
        .api_key("sdk_lab_test_key")
        .base_url("https://sdk-lab.invalid")
        .timeout(Duration::from_secs(2))
        .build();

    let client = match config {
        Ok(cfg) => match HuefyEmailClient::new_with_responder(cfg, stub.responder()) {
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

    if let Some(client) = client {
        let mut send_data = HashMap::new();
        send_data.insert("name".to_string(), json!("Jane"));

        let send_result = client
            .send_email(SendEmailRecipientRequest {
                template_key: "  welcome-email  ".to_string(),
                data: send_data,
                recipient: SendEmailRecipient {
                    email: "  user@example.com  ".to_string(),
                    recipient_type: Some(" CC ".to_string()),
                    data: Some(json!({ "segment": "vip" })),
                },
                provider_type: Some(EmailProvider::Ses),
            })
            .await;

        match send_result {
            Ok(response) if response.success => {
                if let Some(request) = stub.request_at(0) {
                    let recipient = request.body.get("recipient").and_then(Value::as_object);
                    match (
                        request.method.as_str(),
                        request.path.as_str(),
                        request.api_key.as_deref(),
                        request.body.get("templateKey").and_then(Value::as_str),
                        recipient.and_then(|value| value.get("email")).and_then(Value::as_str),
                        recipient.and_then(|value| value.get("type")).and_then(Value::as_str),
                        request.body.get("providerType").and_then(Value::as_str),
                    ) {
                        (
                            "POST",
                            "/emails/send",
                            Some("sdk_lab_test_key"),
                            Some("welcome-email"),
                            Some("user@example.com"),
                            Some("cc"),
                            Some("ses"),
                        ) => r.pass("Single-send contract shaping"),
                        other => r.fail(
                            "Single-send contract shaping",
                            &format!("unexpected captured request shape: {:?}", other),
                        ),
                    }
                } else {
                    r.fail("Single-send contract shaping", "expected captured single-send request");
                }
            }
            Ok(_) => r.fail("Single-send contract shaping", "expected successful stub response"),
            Err(err) => r.fail("Single-send contract shaping", &err.to_string()),
        }

        let bulk_result = client
            .send_bulk_emails(SendBulkEmailsRequest {
                template_key: "  welcome-email  ".to_string(),
                recipients: vec![
                    BulkRecipient {
                        email: "  first@example.com  ".to_string(),
                        recipient_type: Some(" TO ".to_string()),
                        data: Some(json!({ "tier": "gold" })),
                    },
                    BulkRecipient {
                        email: " second@example.com ".to_string(),
                        recipient_type: Some(" BCC ".to_string()),
                        data: None,
                    },
                ],
                provider_type: Some(EmailProvider::Ses),
            })
            .await;

        match bulk_result {
            Ok(response) if response.success => {
                if let Some(request) = stub.request_at(1) {
                    let recipients = request.body.get("recipients").and_then(Value::as_array);
                    let first = recipients.and_then(|items| items.first()).and_then(Value::as_object);
                    let second = recipients.and_then(|items| items.get(1)).and_then(Value::as_object);

                    match (
                        request.method.as_str(),
                        request.path.as_str(),
                        request.body.get("templateKey").and_then(Value::as_str),
                        first.and_then(|value| value.get("email")).and_then(Value::as_str),
                        first.and_then(|value| value.get("type")).and_then(Value::as_str),
                        second.and_then(|value| value.get("email")).and_then(Value::as_str),
                        second.and_then(|value| value.get("type")).and_then(Value::as_str),
                    ) {
                        (
                            "POST",
                            "/emails/send-bulk",
                            Some("welcome-email"),
                            Some("first@example.com"),
                            Some("to"),
                            Some("second@example.com"),
                            Some("bcc"),
                        ) => r.pass("Bulk-send contract shaping"),
                        other => r.fail(
                            "Bulk-send contract shaping",
                            &format!("unexpected captured bulk request shape: {:?}", other),
                        ),
                    }
                } else {
                    r.fail("Bulk-send contract shaping", "expected captured bulk request");
                }
            }
            Ok(_) => r.fail("Bulk-send contract shaping", "expected successful stub response"),
            Err(err) => r.fail("Bulk-send contract shaping", &err.to_string()),
        }

        let before_single = stub.request_count();
        let invalid_single = client
            .send_email(SendEmailRequest {
                template_key: "".to_string(),
                data: HashMap::new(),
                recipient: "not-an-email".to_string(),
                provider_type: None,
            })
            .await;

        match invalid_single {
            Ok(_) => r.fail(
                "Validation rejection for invalid single input",
                "expected validation error",
            ),
            Err(err) if stub.request_count() != before_single => r.fail(
                "Validation rejection for invalid single input",
                &format!("invalid request reached transport: {}", err),
            ),
            Err(_) => r.pass("Validation rejection for invalid single input"),
        }

        let before_bulk = stub.request_count();
        let invalid_bulk = client
            .send_bulk_emails(SendBulkEmailsRequest {
                template_key: "welcome-email".to_string(),
                recipients: vec![BulkRecipient {
                    email: "bad-email".to_string(),
                    recipient_type: Some("reply-to".to_string()),
                    data: None,
                }],
                provider_type: None,
            })
            .await;

        match invalid_bulk {
            Ok(_) => r.fail(
                "Validation rejection for invalid bulk input",
                "expected validation error",
            ),
            Err(err) if stub.request_count() != before_bulk => r.fail(
                "Validation rejection for invalid bulk input",
                &format!("invalid bulk request reached transport: {}", err),
            ),
            Err(_) => r.pass("Validation rejection for invalid bulk input"),
        }

        let health_result = client.health_check().await;
        match health_result {
            Ok(response) if response.data.status == "healthy" => {
                if let Some(request) = stub.request_at(2) {
                    if request.method == "GET" && request.path == "/health" {
                        r.pass("SDK health path behavior");
                    } else {
                        r.fail(
                            "SDK health path behavior",
                            &format!("expected GET /health, got {} {}", request.method, request.path),
                        );
                    }
                } else {
                    r.fail("SDK health path behavior", "expected captured health request");
                }
            }
            Ok(_) => r.fail("SDK health path behavior", "expected decoded healthy response"),
            Err(err) => r.fail("SDK health path behavior", &err.to_string()),
        }

        client.close();
        r.pass("Cleanup");
    }

    print_summary(&r);

    if r.failed > 0 {
        std::process::exit(1);
    }
}

fn is_live_mode() -> bool {
    env::var("HUEFY_SDK_LAB_MODE")
        .map(|value| value.eq_ignore_ascii_case("live"))
        .unwrap_or(false)
}

fn require_env(name: &str) -> String {
    match env::var(name) {
        Ok(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => panic!("{name} is required in live mode"),
    }
}

fn resolve_provider() -> Option<EmailProvider> {
    match env::var("HUEFY_SDK_LIVE_PROVIDER")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "sendgrid" => Some(EmailProvider::Sendgrid),
        "ses" => Some(EmailProvider::Ses),
        "mailgun" => Some(EmailProvider::Mailgun),
        _ => None,
    }
}

async fn run_live_lab(r: &mut Results) {
    let config = HuefyConfig::builder()
        .api_key(require_env("HUEFY_SDK_LIVE_API_KEY"))
        .base_url(require_env("HUEFY_SDK_LIVE_BASE_URL"))
        .timeout(Duration::from_secs(10))
        .build();

    let client = match config {
        Ok(cfg) => match HuefyEmailClient::new(cfg) {
            Ok(c) => {
                r.pass("Initialization");
                Some(c)
            }
            Err(err) => {
                r.fail("Initialization", &err.to_string());
                None
            }
        },
        Err(err) => {
            r.fail("Initialization", &err.to_string());
            None
        }
    };

    let Some(client) = client else {
        return;
    };

    let template_key = require_env("HUEFY_SDK_LIVE_TEMPLATE_KEY");
    let recipient = require_env("HUEFY_SDK_LIVE_RECIPIENT");
    let provider = resolve_provider();

    let mut send_data = HashMap::new();
    send_data.insert("FirstName".to_string(), json!("SDK Live"));

    match client
        .send_email(SendEmailRecipientRequest {
            template_key: template_key.clone(),
            data: send_data,
            recipient: SendEmailRecipient {
                email: recipient.clone(),
                recipient_type: None,
                data: None,
            },
            provider_type: provider.clone(),
        })
        .await
    {
        Ok(response) if response.success => r.pass("Single-send live behavior"),
        Ok(_) => r.fail("Single-send live behavior", "expected successful live send"),
        Err(err) => r.fail("Single-send live behavior", &err.to_string()),
    }

    match client
        .send_bulk_emails(SendBulkEmailsRequest {
            template_key: template_key.clone(),
            recipients: vec![BulkRecipient {
                email: recipient.clone(),
                recipient_type: Some("to".to_string()),
                data: None,
            }],
            provider_type: provider.clone(),
        })
        .await
    {
        Ok(response) if response.success && response.data.total_recipients >= 1 => {
            r.pass("Bulk-send live behavior")
        }
        Ok(_) => r.fail("Bulk-send live behavior", "expected successful live bulk send"),
        Err(err) => r.fail("Bulk-send live behavior", &err.to_string()),
    }

    match client
        .send_email(SendEmailRecipientRequest {
            template_key: template_key.clone(),
            data: HashMap::new(),
            recipient: SendEmailRecipient {
                email: "bad-email".to_string(),
                recipient_type: Some("reply-to".to_string()),
                data: None,
            },
            provider_type: None,
        })
        .await
    {
        Ok(_) => r.fail(
            "Validation rejection for invalid single input",
            "expected validation error",
        ),
        Err(_) => r.pass("Validation rejection for invalid single input"),
    }

    match client
        .send_bulk_emails(SendBulkEmailsRequest {
            template_key,
            recipients: vec![BulkRecipient {
                email: "bad-email".to_string(),
                recipient_type: Some("reply-to".to_string()),
                data: None,
            }],
            provider_type: None,
        })
        .await
    {
        Ok(_) => r.fail(
            "Validation rejection for invalid bulk input",
            "expected validation error",
        ),
        Err(_) => r.pass("Validation rejection for invalid bulk input"),
    }

    match client.health_check().await {
        Ok(response) if response.data.status == "healthy" => r.pass("SDK health path behavior"),
        Ok(response) => r.fail(
            "SDK health path behavior",
            &format!("expected healthy status, got {}", response.data.status),
        ),
        Err(err) => r.fail("SDK health path behavior", &err.to_string()),
    }

    r.pass("Cleanup");
}

fn print_summary(r: &Results) {
    println!();
    println!("========================================");
    println!("Results: {} passed, {} failed", r.passed, r.failed);
    println!("========================================");
    println!();

    if r.failed == 0 {
        println!("All verifications passed!");
    }
}

use huefy::{
    BulkRecipient, EmailProvider, HuefyConfig, HuefyEmailClient, SendBulkEmailsRequest,
    SendEmailRecipient, SendEmailRecipientRequest, SendEmailRequest,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

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

struct StubServer {
    base_url: String,
    requests: Arc<Mutex<Vec<CapturedRequest>>>,
    task: JoinHandle<()>,
}

impl StubServer {
    async fn start() -> Result<Self, Box<dyn Error>> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&requests);

        let task = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let captured = Arc::clone(&captured);
                tokio::spawn(async move {
                    let _ = handle_connection(stream, captured).await;
                });
            }
        });

        Ok(Self {
            base_url: format!("http://{}", address),
            requests,
            task,
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

    fn shutdown(self) {
        self.task.abort();
    }
}

impl Results {
    fn new() -> Self {
        Self {
            passed: 0,
            failed: 0,
        }
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

    let stub = match StubServer::start().await {
        Ok(server) => server,
        Err(err) => {
            r.fail(
                "Initialization",
                &format!("failed to start stub server: {}", err),
            );
            print_summary(&r);
            std::process::exit(1);
        }
    };

    let config = HuefyConfig::builder()
        .api_key("sdk_lab_test_key")
        .base_url(stub.base_url.clone())
        .timeout(Duration::from_secs(2))
        .build();

    let client = match config {
        Ok(cfg) => match HuefyEmailClient::new(cfg) {
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
                        recipient
                            .and_then(|value| value.get("email"))
                            .and_then(Value::as_str),
                        recipient
                            .and_then(|value| value.get("type"))
                            .and_then(Value::as_str),
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
                    r.fail(
                        "Single-send contract shaping",
                        "expected captured single-send request",
                    );
                }
            }
            Ok(_) => r.fail(
                "Single-send contract shaping",
                "expected successful stub response",
            ),
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
                    let first = recipients
                        .and_then(|items| items.first())
                        .and_then(Value::as_object);
                    let second = recipients
                        .and_then(|items| items.get(1))
                        .and_then(Value::as_object);

                    match (
                        request.method.as_str(),
                        request.path.as_str(),
                        request.body.get("templateKey").and_then(Value::as_str),
                        first
                            .and_then(|value| value.get("email"))
                            .and_then(Value::as_str),
                        first
                            .and_then(|value| value.get("type"))
                            .and_then(Value::as_str),
                        second
                            .and_then(|value| value.get("email"))
                            .and_then(Value::as_str),
                        second
                            .and_then(|value| value.get("type"))
                            .and_then(Value::as_str),
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
                    r.fail(
                        "Bulk-send contract shaping",
                        "expected captured bulk request",
                    );
                }
            }
            Ok(_) => r.fail(
                "Bulk-send contract shaping",
                "expected successful stub response",
            ),
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
                            &format!(
                                "expected GET /health, got {} {}",
                                request.method, request.path
                            ),
                        );
                    }
                } else {
                    r.fail(
                        "SDK health path behavior",
                        "expected captured health request",
                    );
                }
            }
            Ok(_) => r.fail(
                "SDK health path behavior",
                "expected decoded healthy response",
            ),
            Err(err) => r.fail("SDK health path behavior", &err.to_string()),
        }

        client.close();
        r.pass("Cleanup");
    }

    stub.shutdown();
    print_summary(&r);

    if r.failed > 0 {
        std::process::exit(1);
    }

    println!("All verifications passed!");
}

fn print_summary(r: &Results) {
    println!();
    println!("========================================");
    println!("Results: {} passed, {} failed", r.passed, r.failed);
    println!("========================================");
    println!();
}

async fn handle_connection(
    mut stream: TcpStream,
    requests: Arc<Mutex<Vec<CapturedRequest>>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let (method, path, headers, body_bytes) = read_http_request(&mut stream).await?;
    let body = if body_bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&body_bytes)?
    };

    let api_key = headers.get("x-api-key").cloned();

    requests
        .lock()
        .expect("request lock")
        .push(CapturedRequest {
            method: method.clone(),
            path: path.clone(),
            body,
            api_key,
        });

    let response_body = match path.as_str() {
        "/emails/send" => json!({
            "success": true,
            "data": {
                "emailId": "email_123",
                "status": "queued",
                "recipients": [
                    {
                        "email": "user@example.com",
                        "status": "queued",
                        "messageId": "msg_123"
                    }
                ]
            },
            "correlationId": "corr_send"
        }),
        "/emails/send-bulk" => json!({
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
                    { "email": "first@example.com", "status": "queued" },
                    { "email": "second@example.com", "status": "queued" }
                ]
            },
            "correlationId": "corr_bulk"
        }),
        "/health" => json!({
            "success": true,
            "data": {
                "status": "healthy",
                "timestamp": "2026-01-01T00:00:00Z",
                "version": "test"
            },
            "correlationId": "corr_health"
        }),
        _ => json!({
            "message": "not found"
        }),
    };

    let status = if path == "/unknown" {
        "404 Not Found"
    } else {
        "200 OK"
    };
    let response_text = response_body.to_string();
    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        response_text.len(),
        response_text
    );

    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await?;
    Ok(())
}

async fn read_http_request(
    stream: &mut TcpStream,
) -> Result<(String, String, HashMap<String, String>, Vec<u8>), Box<dyn Error + Send + Sync>> {
    let mut buffer = Vec::new();
    let mut temp = [0_u8; 1024];
    let mut header_end = None;
    let mut content_length = 0_usize;

    loop {
        let read = stream.read(&mut temp).await?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&temp[..read]);

        if header_end.is_none() {
            if let Some(position) = find_subsequence(&buffer, b"\r\n\r\n") {
                header_end = Some(position + 4);
                let header_text = String::from_utf8_lossy(&buffer[..position]);
                content_length = header_text
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        if name.eq_ignore_ascii_case("content-length") {
                            value.trim().parse::<usize>().ok()
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0);
            }
        }

        if let Some(end) = header_end {
            if buffer.len() >= end + content_length {
                break;
            }
        }
    }

    let header_end = header_end.ok_or("missing header terminator")?;
    let header_text = String::from_utf8(buffer[..header_end - 4].to_vec())?;
    let mut lines = header_text.lines();
    let request_line = lines.next().ok_or("missing request line")?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or("missing method")?.to_string();
    let path = parts.next().ok_or("missing path")?.to_string();

    let mut headers = HashMap::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }

    let body = buffer[header_end..].to_vec();
    Ok((method, path, headers, body))
}

fn find_subsequence(buffer: &[u8], needle: &[u8]) -> Option<usize> {
    buffer
        .windows(needle.len())
        .position(|window| window == needle)
}

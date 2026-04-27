use serde_json::json;
use std::collections::HashMap;

use huefy::validators::email::{
    validate_bulk_count, validate_email, validate_email_data, validate_recipient,
    validate_send_email_input, validate_template_key, MAX_BULK_EMAILS, MAX_EMAIL_LENGTH,
    MAX_TEMPLATE_KEY_LENGTH,
};
use huefy::SendEmailRecipient;

#[test]
fn test_validate_email_valid() {
    assert!(validate_email("user@example.com").is_ok());
    assert!(validate_email("user@mail.example.com").is_ok());
    assert!(validate_email("test+tag@domain.co").is_ok());
}

#[test]
fn test_validate_email_empty() {
    let result = validate_email("");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("required"));
}

#[test]
fn test_validate_email_no_at_sign() {
    assert!(validate_email("userexample.com").is_err());
}

#[test]
fn test_validate_email_no_domain() {
    assert!(validate_email("user@").is_err());
}

#[test]
fn test_validate_email_too_long() {
    let long_email = format!("{}@b.co", "a".repeat(MAX_EMAIL_LENGTH));
    let result = validate_email(&long_email);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("maximum length"));
}

#[test]
fn test_validate_template_key_valid() {
    assert!(validate_template_key("welcome-email").is_ok());
    assert!(validate_template_key("a").is_ok());
}

#[test]
fn test_validate_template_key_empty() {
    assert!(validate_template_key("").is_err());
}

#[test]
fn test_validate_template_key_whitespace_only() {
    assert!(validate_template_key("   ").is_err());
}

#[test]
fn test_validate_template_key_too_long() {
    let long_key = "a".repeat(MAX_TEMPLATE_KEY_LENGTH + 1);
    let result = validate_template_key(&long_key);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("maximum length"));
}

#[test]
fn test_validate_template_key_at_max() {
    let key = "a".repeat(MAX_TEMPLATE_KEY_LENGTH);
    assert!(validate_template_key(&key).is_ok());
}

#[test]
fn test_validate_email_data_valid() {
    let data = HashMap::from([("name".to_string(), json!("John"))]);
    assert!(validate_email_data(Some(&data)).is_ok());
}

#[test]
fn test_validate_email_data_none() {
    assert!(validate_email_data(None).is_err());
}

#[test]
fn test_validate_email_data_empty() {
    let data = HashMap::new();
    assert!(validate_email_data(Some(&data)).is_ok());
}

#[test]
fn test_validate_bulk_count_valid() {
    assert!(validate_bulk_count(1).is_ok());
    assert!(validate_bulk_count(50).is_ok());
    assert!(validate_bulk_count(MAX_BULK_EMAILS).is_ok());
}

#[test]
fn test_validate_bulk_count_zero() {
    assert!(validate_bulk_count(0).is_err());
}

#[test]
fn test_validate_bulk_count_over_limit() {
    let result = validate_bulk_count(MAX_BULK_EMAILS + 1);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("maximum"));
}

#[test]
fn test_validate_send_email_input_valid() {
    let data = HashMap::from([("name".to_string(), json!("John"))]);
    let errors = validate_send_email_input("welcome", Some(&data), "user@test.com", None);
    assert!(errors.is_empty());
}

#[test]
fn test_validate_send_email_input_all_invalid() {
    let errors = validate_send_email_input("", None, "bad", None);
    assert!(errors.len() >= 3);
}

#[test]
fn test_validate_send_email_input_partial_invalid() {
    let data = HashMap::from([("name".to_string(), json!("John"))]);
    let errors = validate_send_email_input("welcome", Some(&data), "bad", None);
    assert_eq!(errors.len(), 1);
}

#[test]
fn test_validate_recipient_object_valid() {
    let recipient = SendEmailRecipient {
        email: "user@test.com".to_string(),
        recipient_type: Some("cc".to_string()),
        data: Some(json!({ "locale": "en" })),
    };
    assert!(validate_recipient(&recipient).is_ok());
}

#[test]
fn test_validate_recipient_object_invalid() {
    let recipient = SendEmailRecipient {
        email: "bad".to_string(),
        recipient_type: None,
        data: None,
    };
    assert!(validate_recipient(&recipient).is_err());
}

#[test]
fn test_validate_recipient_object_invalid_type() {
    let recipient = SendEmailRecipient {
        email: "user@test.com".to_string(),
        recipient_type: Some("reply-to".to_string()),
        data: Some(json!({ "locale": "en" })),
    };
    let result = validate_recipient(&recipient);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("recipient type"));
}

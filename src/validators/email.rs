use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

/// Maximum allowed email address length.
pub const MAX_EMAIL_LENGTH: usize = 254;

/// Maximum allowed template key length.
pub const MAX_TEMPLATE_KEY_LENGTH: usize = 100;

/// Maximum number of emails in a single bulk request.
pub const MAX_BULK_EMAILS: usize = 1000;

static EMAIL_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap());

/// Validates a recipient email address.
///
/// Returns `Ok(())` on success or an error message on failure.
pub fn validate_email(email: &str) -> Result<(), String> {
    if email.is_empty() {
        return Err("recipient email is required".to_string());
    }

    let trimmed = email.trim();

    if trimmed.len() > MAX_EMAIL_LENGTH {
        return Err(format!(
            "email exceeds maximum length of {} characters",
            MAX_EMAIL_LENGTH
        ));
    }

    if !EMAIL_REGEX.is_match(trimmed) {
        return Err(format!("invalid email address: {}", trimmed));
    }

    Ok(())
}

/// Validates a template key.
///
/// Returns `Ok(())` on success or an error message on failure.
pub fn validate_template_key(key: &str) -> Result<(), String> {
    if key.is_empty() {
        return Err("template key is required".to_string());
    }

    let trimmed = key.trim();

    if trimmed.is_empty() {
        return Err("template key cannot be empty".to_string());
    }

    if trimmed.len() > MAX_TEMPLATE_KEY_LENGTH {
        return Err(format!(
            "template key exceeds maximum length of {} characters",
            MAX_TEMPLATE_KEY_LENGTH
        ));
    }

    Ok(())
}

/// Validates template data.
///
/// Returns `Ok(())` on success or an error message on failure.
pub fn validate_email_data(data: Option<&HashMap<String, String>>) -> Result<(), String> {
    if data.is_none() {
        return Err("template data is required".to_string());
    }
    Ok(())
}

/// Validates the count of emails in a bulk request.
///
/// Returns `Ok(())` on success or an error message on failure.
pub fn validate_bulk_count(count: usize) -> Result<(), String> {
    if count == 0 {
        return Err("at least one email is required".to_string());
    }
    if count > MAX_BULK_EMAILS {
        return Err(format!(
            "maximum of {} emails per bulk request",
            MAX_BULK_EMAILS
        ));
    }
    Ok(())
}

/// Validates all inputs for sending a single email.
///
/// Returns a vector of error messages. Empty if all inputs are valid.
pub fn validate_send_email_input(
    template_key: &str,
    data: Option<&HashMap<String, String>>,
    recipient: &str,
) -> Vec<String> {
    let mut errors = Vec::new();

    if let Err(e) = validate_template_key(template_key) {
        errors.push(e);
    }
    if let Err(e) = validate_email_data(data) {
        errors.push(e);
    }
    if let Err(e) = validate_email(recipient) {
        errors.push(e);
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_email_valid() {
        assert!(validate_email("user@example.com").is_ok());
    }

    #[test]
    fn test_validate_email_empty() {
        assert!(validate_email("").is_err());
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
        let long_email = format!("{}@b.co", "a".repeat(250));
        assert!(validate_email(&long_email).is_err());
    }

    #[test]
    fn test_validate_template_key_valid() {
        assert!(validate_template_key("welcome-email").is_ok());
    }

    #[test]
    fn test_validate_template_key_empty() {
        assert!(validate_template_key("").is_err());
    }

    #[test]
    fn test_validate_template_key_too_long() {
        let long_key = "a".repeat(101);
        assert!(validate_template_key(&long_key).is_err());
    }

    #[test]
    fn test_validate_template_key_at_max() {
        let key = "a".repeat(100);
        assert!(validate_template_key(&key).is_ok());
    }

    #[test]
    fn test_validate_bulk_count_valid() {
        assert!(validate_bulk_count(10).is_ok());
    }

    #[test]
    fn test_validate_bulk_count_zero() {
        assert!(validate_bulk_count(0).is_err());
    }

    #[test]
    fn test_validate_bulk_count_over_limit() {
        assert!(validate_bulk_count(1001).is_err());
    }

    #[test]
    fn test_validate_bulk_count_at_limit() {
        assert!(validate_bulk_count(1000).is_ok());
    }

    #[test]
    fn test_validate_send_email_input_valid() {
        let data = HashMap::from([("name".to_string(), "John".to_string())]);
        let errors = validate_send_email_input("tpl", Some(&data), "user@test.com");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_send_email_input_invalid() {
        let errors = validate_send_email_input("", None, "bad");
        assert!(errors.len() >= 3);
    }
}

use regex::Regex;
use std::sync::LazyLock;

/// Compiled regex patterns for detecting sensitive data in error messages.
static SANITIZE_PATTERNS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        // API keys
        (
            Regex::new(r"(?i)(api[_-]?key|apikey|x-api-key)[=:\s]+\S+").unwrap(),
            "$1=[REDACTED]",
        ),
        // Bearer tokens
        (Regex::new(r"(?i)(bearer\s+)\S+").unwrap(), "$1[REDACTED]"),
        // Authorization headers
        (
            Regex::new(r"(?i)(authorization)[=:\s]+\S+").unwrap(),
            "$1=[REDACTED]",
        ),
        // Email addresses
        (
            Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap(),
            "[EMAIL_REDACTED]",
        ),
        // Generic secret / password / token fields
        (
            Regex::new(r"(?i)(password|secret|token|credential)[=:\s]+\S+").unwrap(),
            "$1=[REDACTED]",
        ),
    ]
});

/// Redacts sensitive information from an error message.
///
/// Applies a set of regex replacements to strip API keys, bearer tokens,
/// emails, passwords, and similar secrets from the input string.
pub fn sanitize_error_message(message: &str) -> String {
    let mut result = message.to_string();
    for (pattern, replacement) in SANITIZE_PATTERNS.iter() {
        result = pattern.replace_all(&result, *replacement).to_string();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_api_key() {
        let msg = "Error with api_key=sk_live_abc123xyz";
        let sanitized = sanitize_error_message(msg);
        assert!(!sanitized.contains("sk_live_abc123xyz"));
        assert!(sanitized.contains("[REDACTED]"));
    }

    #[test]
    fn test_sanitize_bearer_token() {
        let msg = "Auth header: Bearer eyJhbGciOiJIUzI1NiJ9.xyz";
        let sanitized = sanitize_error_message(msg);
        assert!(!sanitized.contains("eyJhbGciOiJIUzI1NiJ9"));
        assert!(sanitized.contains("[REDACTED]"));
    }

    #[test]
    fn test_sanitize_email() {
        let msg = "Failed to send to user@example.com";
        let sanitized = sanitize_error_message(msg);
        assert!(!sanitized.contains("user@example.com"));
        assert!(sanitized.contains("[EMAIL_REDACTED]"));
    }

    #[test]
    fn test_no_false_positive() {
        let msg = "Connection refused on port 8080";
        let sanitized = sanitize_error_message(msg);
        assert_eq!(sanitized, msg);
    }
}

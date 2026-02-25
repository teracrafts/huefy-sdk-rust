use regex::Regex;
use std::sync::LazyLock;

/// Known field names that commonly contain personally identifiable information.
static PII_FIELD_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?i)^(email|e[-_]?mail)$").unwrap(),
        Regex::new(r"(?i)^(phone|mobile|cell|fax)").unwrap(),
        Regex::new(r"(?i)(first|last|full)[_-]?name").unwrap(),
        Regex::new(r"(?i)^(ssn|social[_-]?security)").unwrap(),
        Regex::new(r"(?i)^(address|street|city|zip|postal)").unwrap(),
        Regex::new(r"(?i)(date[_-]?of[_-]?birth|dob|birthday)").unwrap(),
        Regex::new(r"(?i)^(passport|driver[_-]?license|national[_-]?id)").unwrap(),
        Regex::new(r"(?i)(credit[_-]?card|card[_-]?number|cvv|ccn)").unwrap(),
        Regex::new(r"(?i)^(ip[_-]?address|user[_-]?agent)$").unwrap(),
    ]
});

/// Value-level patterns that look like PII regardless of the field name.
static PII_VALUE_PATTERNS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        (
            Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap(),
            "email address",
        ),
        (
            Regex::new(r"\b\d{3}[-.]?\d{2}[-.]?\d{4}\b").unwrap(),
            "SSN",
        ),
        (
            Regex::new(r"\b\d{4}[- ]?\d{4}[- ]?\d{4}[- ]?\d{4}\b").unwrap(),
            "credit card number",
        ),
        (
            Regex::new(r"\b\+?1?[-.\s]?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b").unwrap(),
            "phone number",
        ),
    ]
});

/// Checks whether `field_name` is likely to contain PII.
pub fn is_potential_pii_field(field_name: &str) -> bool {
    PII_FIELD_PATTERNS
        .iter()
        .any(|pattern| pattern.is_match(field_name))
}

/// A single PII detection result.
#[derive(Debug, Clone)]
pub struct PiiDetection {
    /// The type of PII detected (e.g., "email address", "SSN").
    pub pii_type: String,
    /// The field in which the PII was found, if applicable.
    pub field: Option<String>,
}

/// Scans a set of key-value pairs for potential PII.
///
/// Returns a list of [`PiiDetection`] entries describing each finding.
pub fn detect_potential_pii(fields: &[(&str, &str)]) -> Vec<PiiDetection> {
    let mut detections = Vec::new();

    for (key, value) in fields {
        // Check field name
        if is_potential_pii_field(key) {
            detections.push(PiiDetection {
                pii_type: format!("PII field name: {}", key),
                field: Some(key.to_string()),
            });
        }

        // Check value patterns
        for (pattern, pii_type) in PII_VALUE_PATTERNS.iter() {
            if pattern.is_match(value) {
                detections.push(PiiDetection {
                    pii_type: pii_type.to_string(),
                    field: Some(key.to_string()),
                });
            }
        }
    }

    detections
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pii_field_detection() {
        assert!(is_potential_pii_field("email"));
        assert!(is_potential_pii_field("Email"));
        assert!(is_potential_pii_field("first_name"));
        assert!(is_potential_pii_field("phone"));
        assert!(is_potential_pii_field("ssn"));
        assert!(!is_potential_pii_field("status"));
        assert!(!is_potential_pii_field("count"));
    }

    #[test]
    fn test_value_detection() {
        let fields = vec![
            ("contact", "user@example.com"),
            ("notes", "Call 555-123-4567"),
            ("description", "Nothing sensitive here"),
        ];

        let detections = detect_potential_pii(&fields);
        assert!(detections.iter().any(|d| d.pii_type == "email address"));
        assert!(detections.iter().any(|d| d.pii_type == "phone number"));
    }

    #[test]
    fn test_no_false_positives() {
        let fields = vec![("status", "active"), ("count", "42")];
        let detections = detect_potential_pii(&fields);
        assert!(detections.is_empty());
    }
}

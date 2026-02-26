use huefy::security::hmac_sign::{create_request_signature, sign_payload, verify_signature};
use huefy::security::pii::{detect_potential_pii, is_potential_pii_field};

// ---------------------------------------------------------------------------
// PII detection tests
// ---------------------------------------------------------------------------

#[test]
fn test_pii_field_names_detected() {
    assert!(is_potential_pii_field("email"));
    assert!(is_potential_pii_field("Email"));
    assert!(is_potential_pii_field("e_mail"));
    assert!(is_potential_pii_field("first_name"));
    assert!(is_potential_pii_field("lastName"));
    assert!(is_potential_pii_field("phone"));
    assert!(is_potential_pii_field("ssn"));
    assert!(is_potential_pii_field("address"));
    assert!(is_potential_pii_field("credit_card"));
    assert!(is_potential_pii_field("date_of_birth"));
}

#[test]
fn test_non_pii_fields_not_flagged() {
    assert!(!is_potential_pii_field("status"));
    assert!(!is_potential_pii_field("count"));
    assert!(!is_potential_pii_field("created_at"));
    assert!(!is_potential_pii_field("id"));
    assert!(!is_potential_pii_field("type"));
}

#[test]
fn test_detect_email_in_value() {
    let fields = vec![("notes", "Contact user@example.com for details")];
    let detections = detect_potential_pii(&fields);
    assert!(
        detections.iter().any(|d| d.pii_type == "email address"),
        "Should detect email address in value"
    );
}

#[test]
fn test_detect_phone_in_value() {
    let fields = vec![("message", "Call me at 555-123-4567")];
    let detections = detect_potential_pii(&fields);
    assert!(
        detections.iter().any(|d| d.pii_type == "phone number"),
        "Should detect phone number in value"
    );
}

#[test]
fn test_detect_ssn_in_value() {
    let fields = vec![("data", "SSN is 123-45-6789")];
    let detections = detect_potential_pii(&fields);
    assert!(
        detections.iter().any(|d| d.pii_type == "SSN"),
        "Should detect SSN in value"
    );
}

#[test]
fn test_clean_data_no_detections() {
    let fields = vec![
        ("status", "active"),
        ("count", "42"),
        ("message", "Hello world"),
    ];
    let detections = detect_potential_pii(&fields);
    assert!(detections.is_empty(), "Should not detect PII in clean data");
}

// ---------------------------------------------------------------------------
// HMAC signature tests
// ---------------------------------------------------------------------------

#[test]
fn test_sign_payload_produces_hex() {
    let secret = b"test-secret";
    let payload = b"test-payload";

    let signature = sign_payload(secret, payload).unwrap();
    assert!(!signature.is_empty());
    // Verify it is valid hex
    assert!(hex::decode(&signature).is_ok());
}

#[test]
fn test_sign_payload_deterministic() {
    let secret = b"test-secret";
    let payload = b"test-payload";

    let sig1 = sign_payload(secret, payload).unwrap();
    let sig2 = sign_payload(secret, payload).unwrap();
    assert_eq!(sig1, sig2);
}

#[test]
fn test_sign_payload_different_inputs() {
    let secret = b"test-secret";
    let sig1 = sign_payload(secret, b"payload-a").unwrap();
    let sig2 = sign_payload(secret, b"payload-b").unwrap();
    assert_ne!(sig1, sig2);
}

#[test]
fn test_verify_valid_signature() {
    let secret = b"my-secret";
    let payload = b"important data";

    let signature = sign_payload(secret, payload).unwrap();
    assert!(verify_signature(secret, payload, &signature));
}

#[test]
fn test_verify_rejects_tampered_payload() {
    let secret = b"my-secret";
    let signature = sign_payload(secret, b"original").unwrap();
    assert!(!verify_signature(secret, b"tampered", &signature));
}

#[test]
fn test_verify_rejects_wrong_secret() {
    let payload = b"data";
    let signature = sign_payload(b"secret-a", payload).unwrap();
    assert!(!verify_signature(b"secret-b", payload, &signature));
}

#[test]
fn test_verify_rejects_malformed_signature() {
    assert!(!verify_signature(b"secret", b"data", "not-hex-at-all!!!"));
}

#[test]
fn test_create_request_signature_deterministic() {
    let secret = b"webhook-secret";
    let sig1 =
        create_request_signature(secret, "POST", "/api/v1/emails/send", "{\"to\":\"a\"}", 1000)
            .unwrap();
    let sig2 =
        create_request_signature(secret, "POST", "/api/v1/emails/send", "{\"to\":\"a\"}", 1000)
            .unwrap();
    assert_eq!(sig1, sig2);
}

#[test]
fn test_create_request_signature_varies_with_body() {
    let secret = b"webhook-secret";
    let sig1 =
        create_request_signature(secret, "POST", "/path", r#"{"a":1}"#, 1).unwrap();
    let sig2 =
        create_request_signature(secret, "POST", "/path", r#"{"b":2}"#, 1).unwrap();
    assert_ne!(sig1, sig2);
}

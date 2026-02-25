use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Signs `payload` with `secret` using HMAC-SHA256 and returns the hex-encoded
/// digest.
///
/// # Errors
///
/// Returns `Err` if the HMAC key is invalid (generally should not happen with
/// SHA-256).
pub fn sign_payload(secret: &[u8], payload: &[u8]) -> Result<String, String> {
    let mut mac =
        HmacSha256::new_from_slice(secret).map_err(|e| format!("Invalid HMAC key: {}", e))?;
    mac.update(payload);
    let result = mac.finalize();
    Ok(hex::encode(result.into_bytes()))
}

/// Creates a composite request signature from method, path, body, and
/// timestamp.
///
/// The canonical string is `"{method}\n{path}\n{body}\n{timestamp}"`.
pub fn create_request_signature(
    secret: &[u8],
    method: &str,
    path: &str,
    body: &str,
    timestamp: u64,
) -> Result<String, String> {
    let canonical = format!("{}\n{}\n{}\n{}", method, path, body, timestamp);
    sign_payload(secret, canonical.as_bytes())
}

/// Verifies that `signature` matches the HMAC-SHA256 of `payload` under
/// `secret`.
///
/// Uses constant-time comparison to prevent timing attacks.
pub fn verify_signature(secret: &[u8], payload: &[u8], signature: &str) -> bool {
    let mut mac = match HmacSha256::new_from_slice(secret) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(payload);

    let expected = match hex::decode(signature) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };

    mac.verify_slice(&expected).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_and_verify() {
        let secret = b"my-secret-key";
        let payload = b"hello world";

        let signature = sign_payload(secret, payload).unwrap();
        assert!(!signature.is_empty());
        assert!(verify_signature(secret, payload, &signature));
    }

    #[test]
    fn test_verify_rejects_bad_signature() {
        let secret = b"my-secret-key";
        let payload = b"hello world";

        assert!(!verify_signature(secret, payload, "bad-signature"));
    }

    #[test]
    fn test_verify_rejects_wrong_secret() {
        let secret = b"my-secret-key";
        let wrong_secret = b"wrong-key";
        let payload = b"hello world";

        let signature = sign_payload(secret, payload).unwrap();
        assert!(!verify_signature(wrong_secret, payload, &signature));
    }

    #[test]
    fn test_create_request_signature() {
        let secret = b"webhook-secret";
        let sig = create_request_signature(secret, "POST", "/api/v1/data", "{}", 1700000000)
            .unwrap();
        assert!(!sig.is_empty());

        // Verify it is deterministic
        let sig2 = create_request_signature(secret, "POST", "/api/v1/data", "{}", 1700000000)
            .unwrap();
        assert_eq!(sig, sig2);
    }

    #[test]
    fn test_different_inputs_different_signatures() {
        let secret = b"webhook-secret";
        let sig1 = create_request_signature(secret, "POST", "/api/v1/a", "{}", 1).unwrap();
        let sig2 = create_request_signature(secret, "POST", "/api/v1/b", "{}", 1).unwrap();
        assert_ne!(sig1, sig2);
    }
}

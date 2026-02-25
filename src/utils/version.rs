/// The current version of the Huefy Rust SDK.
pub const SDK_VERSION: &str = "1.0.0";

/// Returns the full user-agent string for this SDK.
pub fn user_agent() -> String {
    format!("huefy-rust/{}", SDK_VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_not_empty() {
        assert!(!SDK_VERSION.is_empty());
    }

    #[test]
    fn test_user_agent_contains_version() {
        let ua = user_agent();
        assert!(ua.contains(SDK_VERSION));
        assert!(ua.starts_with("huefy-rust/"));
    }
}

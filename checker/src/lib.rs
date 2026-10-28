//! Certificate checker (bennett-isa-spec section 12). Skeleton.

/// Check a certificate document. Fail loud: not implemented yet.
pub fn check(cert_json: &str) -> Result<(), String> {
    if cert_json.trim().is_empty() {
        return Err("certificate is empty".to_string());
    }
    Err("bennett-cert-checker: not implemented (skeleton)".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty() {
        assert!(check("").is_err());
    }

    #[test]
    fn skeleton_fails_loudly() {
        assert!(check("{}").unwrap_err().contains("not implemented"));
    }
}

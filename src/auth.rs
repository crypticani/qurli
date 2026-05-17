pub fn normalize_auth_header(auth: &str) -> Option<String> {
    let auth = auth.trim();
    if auth.is_empty() {
        return None;
    }

    if auth.to_lowercase().starts_with("basic ") || auth.to_lowercase().starts_with("bearer ") {
        Some(auth.to_string())
    } else {
        Some(format!("Bearer {auth}"))
    }
}

pub fn is_secret_name(name: &str) -> bool {
    let name = name.to_lowercase();
    [
        "token",
        "access_token",
        "refresh_token",
        "password",
        "secret",
        "api_key",
        "authorization",
    ]
    .iter()
    .any(|marker| name.contains(marker))
}

pub fn mask_secret(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    match chars.len() {
        0 => String::new(),
        1..=8 => "********".to_string(),
        len => {
            let start: String = chars.iter().take(3).collect();
            let end: String = chars.iter().skip(len - 3).collect();
            format!("{start}...{end}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_auth_values() {
        assert_eq!(
            normalize_auth_header("abc123"),
            Some("Bearer abc123".to_string())
        );
        assert_eq!(
            normalize_auth_header("Bearer abc123"),
            Some("Bearer abc123".to_string())
        );
        assert_eq!(normalize_auth_header("  "), None);
    }

    #[test]
    fn detects_and_masks_secret_names() {
        assert!(is_secret_name("access_token"));
        assert!(is_secret_name("API_KEY"));
        assert!(!is_secret_name("user_id"));
        assert_eq!(mask_secret("abc123xyz"), "abc...xyz");
        assert_eq!(mask_secret("short"), "********");
    }
}

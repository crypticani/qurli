use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResponseState {
    pub status: u16,
    pub status_text: String,
    pub duration: u64,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

pub fn format_json_body(body: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(body) {
        Ok(json) => serde_json::to_string_pretty(&json).unwrap_or_else(|_| body.to_string()),
        Err(_) => body.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_json_response_body() {
        let formatted = format_json_body(r#"{"access_token":"abc123","user":{"id":42}}"#);
        assert!(formatted.contains("\n"));
        assert!(formatted.contains(r#""access_token": "abc123""#));
    }

    #[test]
    fn leaves_non_json_response_body_unchanged() {
        assert_eq!(format_json_body("plain text"), "plain text");
    }
}

use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client,
};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::{auth::normalize_auth_header, response::ResponseState};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Method::Get => write!(f, "GET"),
            Method::Post => write!(f, "POST"),
            Method::Put => write!(f, "PUT"),
            Method::Patch => write!(f, "PATCH"),
            Method::Delete => write!(f, "DELETE"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestInput {
    pub method: Method,
    pub url: String,
    pub headers: String,
    pub body: String,
    pub auth: String,
}

impl RequestInput {
    pub fn new(method: Method, url: String, headers: String, body: String, auth: String) -> Self {
        Self {
            method,
            url,
            headers,
            body,
            auth,
        }
    }
}

pub fn normalize_url(url: &str) -> String {
    let url = url.trim();
    if url.is_empty() {
        "http://localhost".to_string()
    } else if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("https://{url}")
    }
}

pub fn parse_headers(headers_str: &str) -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::new();

    for (line_number, line) in headers_str.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let Some((key, value)) = line.split_once(':') else {
            return Err(format!("Header line {} is missing ':'", line_number + 1));
        };

        let name = HeaderName::from_bytes(key.trim().as_bytes())
            .map_err(|err| format!("Invalid header name on line {}: {err}", line_number + 1))?;
        let value = HeaderValue::from_str(value.trim())
            .map_err(|err| format!("Invalid header value on line {}: {err}", line_number + 1))?;
        headers.insert(name, value);
    }

    Ok(headers)
}

pub async fn execute_request_input(input: RequestInput) -> ResponseState {
    let start_time = Instant::now();

    let url = normalize_url(&input.url);
    let mut headers = match parse_headers(&input.headers) {
        Ok(headers) => headers,
        Err(err) => return error_response(start_time, err),
    };

    if let Some(auth_value) = normalize_auth_header(&input.auth) {
        match HeaderValue::from_str(&auth_value) {
            Ok(value) => {
                headers.insert(reqwest::header::AUTHORIZATION, value);
            }
            Err(err) => {
                return error_response(start_time, format!("Invalid Authorization header: {err}"));
            }
        }
    }

    let client = match Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            return error_response(start_time, format!("Failed to build HTTP client: {err}"))
        }
    };

    let req = match input.method {
        Method::Get => client.get(&url),
        Method::Post => client.post(&url),
        Method::Put => client.put(&url),
        Method::Patch => client.patch(&url),
        Method::Delete => client.delete(&url),
    };

    let req = req.headers(headers);
    let req = if !input.body.trim().is_empty() {
        req.body(input.body)
    } else {
        req
    };

    match req.send().await {
        Ok(res) => {
            let status = res.status().as_u16();
            let status_text = res.status().to_string();
            let duration = start_time.elapsed().as_millis() as u64;

            let mut res_headers = Vec::new();
            for (name, val) in res.headers() {
                res_headers.push((name.to_string(), val.to_str().unwrap_or("").to_string()));
            }

            let text = res.text().await.unwrap_or_default();
            let is_json = res_headers.iter().any(|(k, v)| {
                k.eq_ignore_ascii_case("content-type") && v.contains("application/json")
            });

            let body = if is_json {
                crate::response::format_json_body(&text)
            } else {
                text
            };

            ResponseState {
                status,
                status_text,
                duration,
                headers: res_headers,
                body,
            }
        }
        Err(err) => error_response(start_time, err.to_string()),
    }
}

pub fn error_response(start_time: Instant, message: impl Into<String>) -> ResponseState {
    ResponseState {
        status: 0,
        status_text: format!("Error: {}", message.into()),
        duration: start_time.elapsed().as_millis() as u64,
        headers: vec![],
        body: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};

    #[test]
    fn request_model_serializes_and_deserializes() {
        let method = Method::Post;
        let json = serde_json::to_string(&method).unwrap();
        assert_eq!(json, r#""Post""#);
        assert_eq!(serde_json::from_str::<Method>(&json).unwrap(), Method::Post);
    }

    #[test]
    fn parses_valid_headers() {
        let headers = parse_headers("Content-Type: application/json\nX-Trace: abc").unwrap();
        assert_eq!(headers[CONTENT_TYPE], "application/json");
        assert_eq!(headers["x-trace"], "abc");
    }

    #[test]
    fn rejects_invalid_header_lines() {
        let err = parse_headers("Missing colon").unwrap_err();
        assert!(err.contains("missing ':'"));
    }

    #[test]
    fn normalizes_urls() {
        assert_eq!(normalize_url("example.com"), "https://example.com");
        assert_eq!(normalize_url("http://localhost"), "http://localhost");
        assert_eq!(normalize_url(""), "http://localhost");
    }

    #[test]
    fn inserts_authorization_header() {
        let mut headers = parse_headers("").unwrap();
        let auth = normalize_auth_header("abc123").unwrap();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth).unwrap());
        assert_eq!(headers[AUTHORIZATION], "Bearer abc123");
    }
}

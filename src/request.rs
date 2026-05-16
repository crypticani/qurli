use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client,
};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::response::ResponseState;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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

pub async fn execute_request(
    method: Method,
    url: String,
    headers_str: String,
    body: String,
    auth: String,
) -> ResponseState {
    let start_time = Instant::now();

    // basic url parse validation
    let url = if url.is_empty() {
        "http://localhost".to_string()
    } else if url.starts_with("http") {
        url
    } else {
        format!("https://{}", url)
    };

    let mut headers = HeaderMap::new();
    for line in headers_str.lines() {
        if let Some((k, v)) = line.split_once(':') {
            if let (Ok(name), Ok(val)) = (
                HeaderName::from_bytes(k.trim().as_bytes()),
                HeaderValue::from_str(v.trim()),
            ) {
                headers.insert(name, val);
            }
        }
    }

    if !auth.trim().is_empty() {
        // Assume Basic auth if it has basic, otherwise assume user just typed the auth value correctly (e.g., Bearer <token>)
        let auth_val = if auth.to_lowercase().starts_with("basic ")
            || auth.to_lowercase().starts_with("bearer ")
        {
            auth.trim().to_string()
        } else {
            // Default to bearer if not explicitly given
            format!("Bearer {}", auth.trim())
        };

        if let Ok(val) = HeaderValue::from_str(&auth_val) {
            headers.insert(reqwest::header::AUTHORIZATION, val);
        }
    }

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_default();

    let req = match method {
        Method::Get => client.get(&url),
        Method::Post => client.post(&url),
        Method::Put => client.put(&url),
        Method::Patch => client.patch(&url),
        Method::Delete => client.delete(&url),
    };

    let req = req.headers(headers);
    let req = if !body.trim().is_empty() {
        req.body(body)
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

            let body_pretty = if is_json {
                match serde_json::from_str::<serde_json::Value>(&text) {
                    Ok(json) => serde_json::to_string_pretty(&json).unwrap_or(text),
                    Err(_) => text,
                }
            } else {
                text
            };

            ResponseState {
                status,
                status_text,
                duration,
                headers: res_headers,
                body: body_pretty,
            }
        }
        Err(e) => ResponseState {
            status: 0,
            status_text: format!("Error: {}", e),
            duration: start_time.elapsed().as_millis() as u64,
            headers: vec![],
            body: String::new(),
        },
    }
}

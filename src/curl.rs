use crate::{auth::normalize_auth_header, request::RequestInput};

pub fn generate_curl(input: &RequestInput) -> String {
    let method = input.method.to_string();
    let url = input.url.trim();
    let display_url = if url.is_empty() {
        "http://localhost"
    } else if url.starts_with("http://") || url.starts_with("https://") {
        url
    } else {
        return generate_curl(&RequestInput {
            url: format!("https://{url}"),
            ..input.clone()
        });
    };

    let mut cmd = format!("curl -X {} '{}'", method, shell_single_quote(display_url));

    for line in input.headers.lines() {
        if let Some((k, v)) = line.split_once(':') {
            cmd.push_str(&format!(
                " \\\n  -H '{}'",
                shell_single_quote(&format!("{}: {}", k.trim(), v.trim()))
            ));
        }
    }

    if let Some(auth_value) = normalize_auth_header(&input.auth) {
        cmd.push_str(&format!(
            " \\\n  -H '{}'",
            shell_single_quote(&format!("Authorization: {auth_value}"))
        ));
    }

    if !input.body.trim().is_empty() {
        cmd.push_str(&format!(" \\\n  -d '{}'", shell_single_quote(&input.body)));
    }

    cmd
}

fn shell_single_quote(value: &str) -> String {
    value.replace('\'', "'\\''")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::Method;

    #[test]
    fn generates_curl_command() {
        let input = RequestInput::new(
            Method::Post,
            "https://api.example.com/login".to_string(),
            "Content-Type: application/json".to_string(),
            r#"{"username":"aniket"}"#.to_string(),
            "abc123".to_string(),
        );

        let curl = generate_curl(&input);
        assert!(curl.contains("curl -X POST 'https://api.example.com/login'"));
        assert!(curl.contains("-H 'Content-Type: application/json'"));
        assert!(curl.contains("-H 'Authorization: Bearer abc123'"));
        assert!(curl.contains(r#"-d '{"username":"aniket"}'"#));
    }
}

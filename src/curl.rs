use crate::app::App;

pub fn generate_curl(app: &App) -> String {
    let method = app.method.to_string();
    let url = app.url_input.lines().join("").trim().to_string();
    let display_url = if url.is_empty() {
        "http://localhost"
    } else {
        &url
    };

    let mut cmd = format!("curl -X {} '{}'", method, display_url);

    for line in app.headers_input.lines() {
        if let Some((k, v)) = line.split_once(':') {
            cmd.push_str(&format!(" \\\n  -H '{}: {}'", k.trim(), v.trim()));
        }
    }

    let auth = app.auth_input.lines().join("\n");
    if !auth.trim().is_empty() {
        let auth_val = if auth.to_lowercase().starts_with("basic ")
            || auth.to_lowercase().starts_with("bearer ")
        {
            auth.trim().to_string()
        } else {
            format!("Bearer {}", auth.trim())
        };
        cmd.push_str(&format!(" \\\n  -H 'Authorization: {}'", auth_val));
    }

    let body = app.body_input.lines().join("\n");
    if !body.trim().is_empty() {
        let escaped_body = body.replace("'", "'\\''");
        cmd.push_str(&format!(" \\\n  -d '{}'", escaped_body));
    }

    cmd
}

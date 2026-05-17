use crate::app::App;
use crate::auth::is_secret_name;
use crate::request::Method;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct HistoryState {
    method: Method,
    url: Vec<String>,
    headers: Vec<String>,
    body: Vec<String>,
    auth: Vec<String>,
    #[serde(default)]
    extraction_rules: Vec<String>,
}

fn get_history_path() -> Option<PathBuf> {
    let mut path = dirs::config_dir()?;
    path.push("qurli");
    if !path.exists() {
        fs::create_dir_all(&path).ok()?;
    }
    path.push("history.json");
    Some(path)
}

pub fn save_history(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let path = get_history_path().ok_or("Could not find config directory")?;
    save_history_to_path(app, &path)
}

fn save_history_to_path(
    app: &App,
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = history_state_from_app(app);
    let json = serde_json::to_string_pretty(&state)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn load_history(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let path = get_history_path().ok_or("Could not find config directory")?;
    load_history_from_path(app, &path)
}

fn load_history_from_path(
    app: &mut App,
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if !path.exists() {
        return Ok(());
    }
    let json = fs::read_to_string(path)?;
    let state: HistoryState = serde_json::from_str(&json)?;

    app.method = state.method;

    app.url_input = tui_textarea::TextArea::new(state.url);
    app.headers_input = tui_textarea::TextArea::new(state.headers);
    app.body_input = tui_textarea::TextArea::new(state.body);
    app.auth_input = tui_textarea::TextArea::new(state.auth);
    app.extraction_input = tui_textarea::TextArea::new(state.extraction_rules);

    Ok(())
}

fn history_state_from_app(app: &App) -> HistoryState {
    HistoryState {
        method: app.method.clone(),
        url: app.url_input.lines().to_vec(),
        headers: app
            .headers_input
            .lines()
            .iter()
            .map(|line| sanitize_header_line(line))
            .collect(),
        body: sanitize_body_lines(app.body_input.lines()),
        auth: app
            .auth_input
            .lines()
            .iter()
            .map(|line| sanitize_auth_line(line))
            .collect(),
        extraction_rules: app.extraction_input.lines().to_vec(),
    }
}

fn sanitize_header_line(line: &str) -> String {
    let Some((name, value)) = line.split_once(':') else {
        return line.to_string();
    };

    if is_secret_name(name) && !value.contains("{{") {
        format!("{}: <redacted>", name.trim())
    } else {
        line.to_string()
    }
}

fn sanitize_auth_line(line: &str) -> String {
    if line.trim().is_empty() || line.contains("{{") {
        line.to_string()
    } else {
        "<redacted>".to_string()
    }
}

fn sanitize_body_lines(lines: &[String]) -> Vec<String> {
    let body = lines.join("\n");
    if body.trim().is_empty() {
        return lines.to_vec();
    }

    let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&body) else {
        return lines.to_vec();
    };

    redact_secret_json_fields(&mut json);
    serde_json::to_string_pretty(&json)
        .map(|body| body.lines().map(ToString::to_string).collect())
        .unwrap_or_else(|_| lines.to_vec())
}

fn redact_secret_json_fields(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, value) in object.iter_mut() {
                if is_secret_name(key) && !value.as_str().is_some_and(|value| value.contains("{{"))
                {
                    *value = serde_json::Value::String("<redacted>".to_string());
                } else {
                    redact_secret_json_fields(value);
                }
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                redact_secret_json_fields(value);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tui_textarea::TextArea;

    #[test]
    fn history_serialization_redacts_direct_secrets_but_keeps_templates() {
        let mut app = App::new();
        app.headers_input = TextArea::new(vec![
            "Authorization: Bearer abc123".to_string(),
            "X-Api-Key: {{api_key}}".to_string(),
        ]);
        app.auth_input = TextArea::new(vec!["abc123".to_string()]);
        app.body_input = TextArea::new(vec![
            r#"{"password":"pass","nested":{"refresh_token":"rt","name":"aniket"}}"#.to_string(),
        ]);
        app.extraction_input = TextArea::new(vec!["token = $.access_token".to_string()]);

        let json = serde_json::to_string(&history_state_from_app(&app)).unwrap();

        assert!(json.contains("Authorization: <redacted>"));
        assert!(json.contains("X-Api-Key: {{api_key}}"));
        assert!(json.contains("<redacted>"));
        assert!(!json.contains("abc123"));
        assert!(!json.contains(r#""pass""#));
        assert!(!json.contains(r#""rt""#));
        assert!(json.contains("token = $.access_token"));
    }

    #[test]
    fn history_persists_and_loads_request_templates() {
        let mut app = App::new();
        app.method = Method::Post;
        app.url_input = TextArea::new(vec!["{{base_url}}/login".to_string()]);
        app.headers_input = TextArea::new(vec!["Authorization: Bearer {{token}}".to_string()]);
        app.body_input = TextArea::new(vec![r#"{"username":"aniket"}"#.to_string()]);
        app.extraction_input = TextArea::new(vec!["token = $.access_token".to_string()]);

        let path = std::env::temp_dir().join(format!(
            "qurli-history-test-{}-{}.json",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        save_history_to_path(&app, &path).unwrap();

        let mut loaded = App::new();
        load_history_from_path(&mut loaded, &path).unwrap();
        let _ = fs::remove_file(path);

        assert_eq!(loaded.method, Method::Post);
        assert_eq!(
            loaded.url_input.lines(),
            &["{{base_url}}/login".to_string()]
        );
        assert_eq!(
            loaded.headers_input.lines(),
            &["Authorization: Bearer {{token}}".to_string()]
        );
        assert_eq!(
            loaded.extraction_input.lines(),
            &["token = $.access_token".to_string()]
        );
    }
}

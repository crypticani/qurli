use crate::app::App;
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
    let state = HistoryState {
        method: app.method.clone(),
        url: app.url_input.lines().to_vec(),
        headers: app.headers_input.lines().to_vec(),
        body: app.body_input.lines().to_vec(),
        auth: app.auth_input.lines().to_vec(),
    };
    let json = serde_json::to_string_pretty(&state)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn load_history(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let path = get_history_path().ok_or("Could not find config directory")?;
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

    Ok(())
}

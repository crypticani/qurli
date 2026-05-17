use arboard::Clipboard;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tui_textarea::TextArea;

use crate::keybindings;
use crate::request::{Method, RequestInput};
use crate::response::ResponseState;
use crate::variables::{self, RuntimeVariable};

#[derive(PartialEq)]
pub enum Mode {
    Normal,
    Insert,
}

#[derive(PartialEq, Clone, Copy)]
pub enum ActivePane {
    Url,
    Headers,
    Body,
    Auth,
    Method,
    Extract,
}

pub struct App<'a> {
    pub mode: Mode,
    pub active_pane: ActivePane,
    pub should_quit: bool,

    pub method: Method,
    pub url_input: TextArea<'a>,
    pub headers_input: TextArea<'a>,
    pub body_input: TextArea<'a>,
    pub auth_input: TextArea<'a>,
    pub extraction_input: TextArea<'a>,

    pub response: Option<ResponseState>,
    pub is_loading: bool,
    pub response_scroll: u16,
    pub variables: HashMap<String, RuntimeVariable>,
    pub status_message: String,
}

impl<'a> App<'a> {
    pub fn new() -> Self {
        Self {
            mode: Mode::Normal,
            active_pane: ActivePane::Url,
            should_quit: false,
            method: Method::Get,
            url_input: textarea("https://api.example.com"),
            headers_input: textarea(
                "Content-Type: application/json\nAuthorization: Bearer {{token}}",
            ),
            body_input: textarea("{\n  \"key\": \"value\"\n}"),
            auth_input: textarea("Bearer <token>"),
            extraction_input: textarea("token = $.access_token\nuser_id = $.user.id"),
            response: None,
            is_loading: false,
            response_scroll: 0,
            variables: HashMap::new(),
            status_message: "Ready.".to_string(),
        }
    }

    pub fn load_history(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        crate::history::load_history(self)
    }

    pub fn save_history(&self) -> Result<(), Box<dyn std::error::Error>> {
        crate::history::save_history(self)
    }

    pub fn handle_key(&mut self, key: KeyEvent, tx: mpsc::Sender<ResponseState>) {
        if keybindings::is_quit(key) {
            self.should_quit = true;
            return;
        }

        match self.mode {
            Mode::Normal => self.handle_normal_key(key, tx),
            Mode::Insert => self.handle_insert_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent, tx: mpsc::Sender<ResponseState>) {
        if key.code == KeyCode::Char('n') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.clear_inputs();
            return;
        }

        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('s') => self.send_request(tx),
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.send_request(tx)
            }
            KeyCode::Char('y') => self.copy_curl(),
            KeyCode::Char('c') => self.copy_response(),
            KeyCode::Char('m') => self.cycle_method(),
            KeyCode::Char('v') => self.active_pane = ActivePane::Extract,
            KeyCode::Char('e') => {
                self.active_pane = ActivePane::Extract;
                self.mode = Mode::Insert;
            }
            KeyCode::Char('x') => self.run_extraction_on_latest_response(),
            KeyCode::Char('h') => {
                self.active_pane = ActivePane::Headers;
                self.mode = Mode::Insert;
            }
            KeyCode::Char('b') => {
                self.active_pane = ActivePane::Body;
                self.mode = Mode::Insert;
            }
            KeyCode::Char('a') => {
                self.active_pane = ActivePane::Auth;
                self.mode = Mode::Insert;
            }
            KeyCode::Char('u') => {
                self.active_pane = ActivePane::Url;
                self.mode = Mode::Insert;
            }
            KeyCode::Enter | KeyCode::Char('i') => {
                self.mode = Mode::Insert;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(res) = &self.response {
                    let max_scroll = (res.body.lines().count() as u16).saturating_sub(1);
                    if self.response_scroll < max_scroll {
                        self.response_scroll += 1;
                    }
                }
            }
            KeyCode::Up | KeyCode::Char('k') if self.response_scroll > 0 => {
                self.response_scroll -= 1;
            }
            KeyCode::Tab => self.next_pane(),
            KeyCode::BackTab => self.prev_pane(),
            _ => {}
        }
    }

    fn handle_insert_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Esc {
            self.mode = Mode::Normal;
            return;
        }

        match self.active_pane {
            ActivePane::Url => {
                self.url_input.input(key);
            }
            ActivePane::Headers => {
                self.headers_input.input(key);
            }
            ActivePane::Body => {
                self.body_input.input(key);
            }
            ActivePane::Auth => {
                self.auth_input.input(key);
            }
            ActivePane::Extract => {
                self.extraction_input.input(key);
            }
            ActivePane::Method => {}
        }
    }

    fn next_pane(&mut self) {
        self.active_pane = match self.active_pane {
            ActivePane::Method => ActivePane::Url,
            ActivePane::Url => ActivePane::Headers,
            ActivePane::Headers => ActivePane::Auth,
            ActivePane::Auth => ActivePane::Body,
            ActivePane::Body => ActivePane::Extract,
            ActivePane::Extract => ActivePane::Method,
        };
    }

    fn prev_pane(&mut self) {
        self.active_pane = match self.active_pane {
            ActivePane::Method => ActivePane::Extract,
            ActivePane::Url => ActivePane::Method,
            ActivePane::Headers => ActivePane::Url,
            ActivePane::Auth => ActivePane::Headers,
            ActivePane::Body => ActivePane::Auth,
            ActivePane::Extract => ActivePane::Body,
        };
    }

    fn cycle_method(&mut self) {
        self.method = match self.method {
            Method::Get => Method::Post,
            Method::Post => Method::Put,
            Method::Put => Method::Patch,
            Method::Patch => Method::Delete,
            Method::Delete => Method::Get,
        };
    }

    fn send_request(&mut self, tx: mpsc::Sender<ResponseState>) {
        if self.is_loading {
            return;
        }

        let input = match self.substituted_request(false) {
            Ok(input) => input,
            Err(unresolved) => {
                self.response = Some(ResponseState {
                    status: 0,
                    status_text: "Error: unresolved variables".to_string(),
                    duration: 0,
                    headers: vec![],
                    body: format!("Missing variables: {}", unresolved.join(", ")),
                });
                self.status_message = "Request blocked: unresolved {{variables}}.".to_string();
                return;
            }
        };

        self.is_loading = true;
        self.status_message = "Sending request...".to_string();
        tokio::spawn(async move {
            let res = crate::request::execute_request_input(input).await;
            let _ = tx.send(res).await;
        });
    }

    pub fn handle_response(&mut self, res: ResponseState) {
        self.is_loading = false;
        self.response = Some(res);
        self.run_extraction_on_latest_response();
        if self.status_message == "Sending request..." {
            self.status_message = "Response received.".to_string();
        }
        self.response_scroll = 0;
    }

    pub fn substituted_request(&self, mask_secrets: bool) -> Result<RequestInput, Vec<String>> {
        let fields = [
            variables::substitute(
                self.url_input.lines().join("").trim(),
                &self.variables,
                mask_secrets,
            ),
            variables::substitute(
                &self.headers_input.lines().join("\n"),
                &self.variables,
                mask_secrets,
            ),
            variables::substitute(
                &self.body_input.lines().join("\n"),
                &self.variables,
                mask_secrets,
            ),
            variables::substitute(
                &self.auth_input.lines().join("\n"),
                &self.variables,
                mask_secrets,
            ),
        ];

        let mut unresolved = fields
            .iter()
            .flat_map(|field| field.unresolved.clone())
            .collect::<Vec<_>>();
        unresolved.sort();
        unresolved.dedup();

        if !unresolved.is_empty() {
            return Err(unresolved);
        }

        Ok(RequestInput::new(
            self.method.clone(),
            fields[0].value.clone(),
            fields[1].value.clone(),
            fields[2].value.clone(),
            fields[3].value.clone(),
        ))
    }

    pub fn extraction_rules_text(&self) -> String {
        self.extraction_input.lines().join("\n")
    }

    pub fn run_extraction_on_latest_response(&mut self) {
        let Some(response) = &self.response else {
            self.status_message = "No response available for extraction.".to_string();
            return;
        };

        if response.status == 0 {
            return;
        }

        let rules = variables::parse_extraction_rules(&self.extraction_rules_text());
        if rules.is_empty() {
            return;
        }

        match variables::apply_extraction_rules(&response.body, &rules, &mut self.variables) {
            Ok(0) => {
                self.status_message = "No extraction rules matched the response.".to_string();
            }
            Ok(count) => {
                self.status_message = format!("Extracted {count} variable(s).");
            }
            Err(err) => {
                self.status_message = err;
            }
        }
    }

    pub fn safe_curl_preview(&self) -> String {
        match self.substituted_request(true) {
            Ok(input) => crate::curl::generate_curl(&input),
            Err(unresolved) => format!("Unresolved variables: {}", unresolved.join(", ")),
        }
    }

    pub fn variable_lines(&self) -> Vec<String> {
        let mut variables = self.variables.values().collect::<Vec<_>>();
        variables.sort_by(|left, right| left.key.cmp(&right.key));

        if variables.is_empty() {
            return vec!["No runtime variables.".to_string()];
        }

        variables
            .into_iter()
            .map(|variable| {
                let value = if variable.secret {
                    crate::auth::mask_secret(&variable.value)
                } else {
                    variable.value.clone()
                };
                format!("{} = {}", variable.key, value)
            })
            .collect()
    }

    fn copy_curl(&self) {
        if let Ok(input) = self.substituted_request(false) {
            let curl_cmd = crate::curl::generate_curl(&input);
            if let Ok(mut clipboard) = Clipboard::new() {
                let _ = clipboard.set_text(curl_cmd);
            }
        }
    }

    fn copy_response(&self) {
        if let Some(res) = &self.response {
            if let Ok(mut clipboard) = Clipboard::new() {
                let _ = clipboard.set_text(res.body.clone());
            }
        }
    }

    fn clear_inputs(&mut self) {
        self.url_input = textarea("https://api.example.com");
        self.headers_input =
            textarea("Content-Type: application/json\nAuthorization: Bearer {{token}}");
        self.body_input = textarea("{\n  \"key\": \"value\"\n}");
        self.auth_input = textarea("Bearer <token>");
        self.response = None;
        self.status_message = "Cleared request inputs. Runtime variables kept.".to_string();
    }
}

fn textarea(placeholder: &str) -> TextArea<'static> {
    let mut textarea = TextArea::default();
    textarea.set_placeholder_text(placeholder);
    textarea
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_variables_into_request_fields() {
        let mut app = App::new();
        app.url_input = TextArea::new(vec!["{{base_url}}/me".to_string()]);
        app.headers_input = TextArea::new(vec!["Authorization: Bearer {{token}}".to_string()]);
        app.variables.insert(
            "base_url".to_string(),
            RuntimeVariable {
                key: "base_url".to_string(),
                value: "https://api.example.com".to_string(),
                secret: false,
            },
        );
        app.variables.insert(
            "token".to_string(),
            RuntimeVariable {
                key: "token".to_string(),
                value: "abc123".to_string(),
                secret: true,
            },
        );

        let input = app.substituted_request(false).unwrap();
        assert_eq!(input.url, "https://api.example.com/me");
        assert!(input.headers.contains("Bearer abc123"));

        let safe = app.substituted_request(true).unwrap();
        assert!(safe.headers.contains("Bearer ********"));
    }

    #[test]
    fn reports_unresolved_variables() {
        let mut app = App::new();
        app.url_input = TextArea::new(vec!["{{missing}}/me".to_string()]);

        assert_eq!(app.substituted_request(false).unwrap_err(), vec!["missing"]);
    }

    #[test]
    fn extracts_variables_after_response() {
        let mut app = App::new();
        app.extraction_input = TextArea::new(vec![
            "token = $.access_token".to_string(),
            "user_id = $.user.id".to_string(),
        ]);
        app.response = Some(ResponseState {
            status: 200,
            status_text: "200 OK".to_string(),
            duration: 10,
            headers: vec![],
            body: r#"{"access_token":"abc123","user":{"id":42}}"#.to_string(),
        });

        app.run_extraction_on_latest_response();

        assert_eq!(app.variables["token"].value, "abc123");
        assert!(app.variables["token"].secret);
        assert_eq!(app.variables["user_id"].value, "42");
    }
}

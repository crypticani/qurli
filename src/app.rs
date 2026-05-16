use arboard::Clipboard;
use crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc;
use tui_textarea::TextArea;

use crate::keybindings;
use crate::request::Method;
use crate::response::ResponseState;

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

    pub response: Option<ResponseState>,
    pub is_loading: bool,
    pub response_scroll: u16,
}

impl<'a> App<'a> {
    pub fn new() -> Self {
        let mut url_input = TextArea::default();
        url_input.set_placeholder_text("https://api.example.com");

        let mut headers_input = TextArea::default();
        headers_input
            .set_placeholder_text("Content-Type: application/json\nAuthorization: Bearer token");

        let mut body_input = TextArea::default();
        body_input.set_placeholder_text("{\n  \"key\": \"value\"\n}");

        let mut auth_input = TextArea::default();
        auth_input.set_placeholder_text("Bearer <token>");

        Self {
            mode: Mode::Normal,
            active_pane: ActivePane::Url,
            should_quit: false,
            method: Method::Get,
            url_input,
            headers_input,
            body_input,
            auth_input,
            response: None,
            is_loading: false,
            response_scroll: 0,
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
        if key.code == KeyCode::Char('n') && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
            self.clear_inputs();
            return;
        }

        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('s') => self.send_request(tx),
            KeyCode::Char('y') => self.copy_curl(),
            KeyCode::Char('c') => self.copy_response(),
            KeyCode::Char('m') => self.cycle_method(),
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
            ActivePane::Method => {} // Method is selected via normal mode
        }
    }

    fn next_pane(&mut self) {
        self.active_pane = match self.active_pane {
            ActivePane::Method => ActivePane::Url,
            ActivePane::Url => ActivePane::Headers,
            ActivePane::Headers => ActivePane::Auth,
            ActivePane::Auth => ActivePane::Body,
            ActivePane::Body => ActivePane::Method,
        };
    }

    fn prev_pane(&mut self) {
        self.active_pane = match self.active_pane {
            ActivePane::Method => ActivePane::Body,
            ActivePane::Url => ActivePane::Method,
            ActivePane::Headers => ActivePane::Url,
            ActivePane::Auth => ActivePane::Headers,
            ActivePane::Body => ActivePane::Auth,
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
        self.is_loading = true;

        let method = self.method.clone();
        let url = self.url_input.lines().join("").trim().to_string();
        let headers = self.headers_input.lines().join("\n");
        let body = self.body_input.lines().join("\n");
        let auth = self.auth_input.lines().join("\n");

        tokio::spawn(async move {
            let res = crate::request::execute_request(method, url, headers, body, auth).await;
            let _ = tx.send(res).await;
        });
    }

    pub fn handle_response(&mut self, res: ResponseState) {
        self.is_loading = false;
        self.response = Some(res);
        self.response_scroll = 0;
    }

    fn copy_curl(&self) {
        let curl_cmd = crate::curl::generate_curl(self);
        if let Ok(mut clipboard) = Clipboard::new() {
            let _ = clipboard.set_text(curl_cmd);
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
        self.url_input = TextArea::default();
        self.url_input.set_placeholder_text("https://api.example.com");

        self.headers_input = TextArea::default();
        self.headers_input
            .set_placeholder_text("Content-Type: application/json\nAuthorization: Bearer token");

        self.body_input = TextArea::default();
        self.body_input.set_placeholder_text("{\n  \"key\": \"value\"\n}");

        self.auth_input = TextArea::default();
        self.auth_input.set_placeholder_text("Bearer <token>");

        self.response = None;
    }
}

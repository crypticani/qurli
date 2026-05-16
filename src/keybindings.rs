use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn is_quit(key: KeyEvent) -> bool {
    // We only quit if it's 'q' in Normal mode, but since main handles it before App,
    // wait, main shouldn't handle `q` blindly because `q` might be typed in a textarea.
    // So we should move `is_quit` to `App` handling, or pass mode to `is_quit`.
    // For safety, let's only do Ctrl+C here, and let App handle 'q' for Normal mode.
    key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
}

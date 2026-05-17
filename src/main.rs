mod app;
mod auth;
mod curl;
mod history;
mod keybindings;
mod request;
mod response;
mod ui;
mod variables;

use app::App;
use crossterm::{
    event::{Event, EventStream},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut cleanup = TerminalCleanup::active();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal).await;

    cleanup.restore(&mut terminal)?;

    result
}

struct TerminalCleanup {
    active: bool,
}

impl TerminalCleanup {
    fn active() -> Self {
        Self { active: true }
    }

    fn restore(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.active {
            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            terminal.show_cursor()?;
            self.active = false;
        }
        Ok(())
    }
}

impl Drop for TerminalCleanup {
    fn drop(&mut self) {
        if self.active {
            let _ = disable_raw_mode();
            let mut stdout = io::stdout();
            let _ = execute!(stdout, LeaveAlternateScreen);
        }
    }
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let _ = app.load_history();

    let (tx, mut rx) = mpsc::channel(32);

    let mut reader = EventStream::new();

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        tokio::select! {
            Some(Ok(evt)) = reader.next() => {
                match evt {
                    Event::Key(key) => {
                        app.handle_key(key, tx.clone());
                        if app.should_quit {
                            let _ = app.save_history();
                            break;
                        }
                    }
                    Event::Resize(..) => {}
                    _ => {}
                }
            }
            Some(res) = rx.recv() => {
                app.handle_response(res);
            }
        }
    }

    Ok(())
}

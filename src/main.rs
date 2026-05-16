mod app;
mod auth;
mod curl;
mod history;
mod keybindings;
mod request;
mod response;
mod ui;

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
    // Terminal initialization
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // App state
    let mut app = App::new();
    let _ = app.load_history();

    // Async channels for network responses
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
                    Event::Resize(..) => {
                        // Handled automatically by terminal.draw
                    }
                    _ => {}
                }
            }
            Some(res) = rx.recv() => {
                app.handle_response(res);
            }
        }
    }

    // Terminal cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

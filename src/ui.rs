use crate::app::{ActivePane, App, Mode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
    Frame,
};

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Main wrapper block
    let wrapper_block = Block::bordered().title(" qurli ");
    let inner_area = wrapper_block.inner(size);
    f.render_widget(wrapper_block, size);

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Top: Method + URL
            Constraint::Length(8), // Middle: Headers, Auth, Body
            Constraint::Length(4), // Curl preview
            Constraint::Min(5),    // Response
        ])
        .split(inner_area);

    draw_top_bar(f, app, main_chunks[0]);
    draw_middle_inputs(f, app, main_chunks[1]);
    draw_curl_preview(f, app, main_chunks[2]);
    draw_response(f, app, main_chunks[3]);
}

fn get_style(app: &App, pane: ActivePane) -> Style {
    if app.active_pane == pane {
        if app.mode == Mode::Insert {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Yellow)
        }
    } else {
        Style::default().fg(Color::White)
    }
}

fn draw_top_bar(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(15), Constraint::Min(20)])
        .split(area);

    let method_style = get_style(app, ActivePane::Method);
    let method_p = Paragraph::new(app.method.to_string())
        .block(Block::bordered().title("Method [m]").style(method_style));
    f.render_widget(method_p, chunks[0]);

    app.url_input.set_block(
        Block::bordered()
            .title("URL [u]")
            .style(get_style(app, ActivePane::Url)),
    );
    f.render_widget(&app.url_input, chunks[1]);
}

fn draw_middle_inputs(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(area);

    app.headers_input.set_block(
        Block::bordered()
            .title("Headers [h]")
            .style(get_style(app, ActivePane::Headers)),
    );
    f.render_widget(&app.headers_input, chunks[0]);

    app.auth_input.set_block(
        Block::bordered()
            .title("Auth [a]")
            .style(get_style(app, ActivePane::Auth)),
    );
    f.render_widget(&app.auth_input, chunks[1]);

    app.body_input.set_block(
        Block::bordered()
            .title("Body [b]")
            .style(get_style(app, ActivePane::Body)),
    );
    f.render_widget(&app.body_input, chunks[2]);
}

fn draw_curl_preview(f: &mut Frame, app: &App, area: Rect) {
    let curl_cmd = crate::curl::generate_curl(app);
    let p = Paragraph::new(curl_cmd)
        .block(Block::bordered().title("Generated curl [y to copy]"))
        .wrap(Wrap { trim: true });
    f.render_widget(p, area);
}

fn draw_response(f: &mut Frame, app: &App, area: Rect) {
    let title = if app.is_loading {
        "Response [Loading...]".to_string()
    } else {
        "Response [j/k to scroll]".to_string()
    };

    let mut content = Vec::new();

    if let Some(res) = &app.response {
        let status_color = match res.status {
            200..=299 => Color::Green,
            300..=399 => Color::Yellow,
            400..=599 => Color::Red,
            _ => Color::White,
        };

        content.push(Line::from(vec![
            Span::raw("Status: "),
            Span::styled(
                format!("{} {}", res.status, res.status_text),
                Style::default().fg(status_color),
            ),
        ]));

        content.push(Line::from(format!("Time: {}ms", res.duration)));
        content.push(Line::from(""));

        for line in res.body.lines() {
            content.push(Line::from(line.to_string()));
        }
    } else {
        content.push(Line::from("Press 's' to send request"));
        content.push(Line::from("Press 'i' or Enter to edit focused pane"));
        content.push(Line::from("Press 'Esc' to exit edit mode"));
        content.push(Line::from("Press 'Tab' to switch panes"));
        content.push(Line::from("Press 'q' to quit"));
    }

    let p = Paragraph::new(content)
        .block(Block::bordered().title(title))
        .scroll((app.response_scroll, 0))
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

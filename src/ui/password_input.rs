use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::{App, PasswordContext};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let remote = app
        .current_pair
        .as_ref()
        .map(|p| p.remote.as_str())
        .unwrap_or("remote");

    let prompt = match app.password_context {
        PasswordContext::Sync => format!("Password for {} (sync):", remote),
        PasswordContext::RemoteList => format!("Password for {} (load remote tree):", remote),
    };

    let masked = "*".repeat(app.password_buffer.len());

    let lines = vec![
        Line::from(Span::styled(prompt, Style::default().fg(Color::Yellow))),
        Line::from(""),
        Line::from(vec![
            Span::styled("> ", Style::default().fg(Color::Cyan)),
            Span::styled(masked, Style::default().fg(Color::White)),
            Span::styled(
                "_",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::SLOW_BLINK),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Enter: submit | Esc: cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let block = Block::default()
        .title(" Authentication ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Red));

    let paragraph = Paragraph::new(lines).block(block);

    let popup = Layout::default()
        .constraints([
            Constraint::Percentage(35),
            Constraint::Min(9),
            Constraint::Percentage(35),
        ])
        .split(area);

    let inner = Layout::default()
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ])
        .split(popup[1]);

    f.render_widget(Clear, inner[1]);
    f.render_widget(paragraph, inner[1]);
}

pub fn handle_key(app: &mut App, key: crossterm::event::KeyEvent) {
    use crossterm::event::{KeyCode, KeyEventKind};
    if key.kind != KeyEventKind::Press {
        return;
    }
    match key.code {
        KeyCode::Enter => app.submit_password(),
        KeyCode::Esc => app.cancel_password(),
        KeyCode::Char(c) => {
            app.password_buffer.push(c);
        }
        KeyCode::Backspace => {
            app.password_buffer.pop();
        }
        _ => {}
    }
}

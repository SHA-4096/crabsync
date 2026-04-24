use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let title = if let Some(pair) = &app.current_pair {
        format!(" Sync Preview (dry-run) - {} ", pair.name)
    } else {
        " Sync Preview ".to_string()
    };

    let mut lines: Vec<Line> = Vec::new();

    if !app.sync_command.is_empty() {
        lines.push(Line::from(Span::styled(
            "Command:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        for line in app.sync_command.lines() {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(Color::White),
            )));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Dry-run output:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
    }

    let auth_required = app.dry_run_output.contains("Authentication required");

    if app.dry_run_output.is_empty() {
        lines.push(Line::from("No changes to sync."));
    } else {
        for line in app.dry_run_output.lines() {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(Color::Yellow),
            )));
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Green)),
        )
        .wrap(Wrap { trim: false })
        .scroll((0, 0));

    f.render_widget(paragraph, area);

    let status = if auth_required {
        "y: confirm sync (password required) | n/Esc: go back | ?: help"
    } else {
        "y: confirm sync | n/Esc: go back | ?: help"
    };
    let status_bar = Paragraph::new(status).style(Style::default().fg(Color::DarkGray));
    let status_area = Rect::new(area.x, area.y + area.height - 1, area.width, 1);
    f.render_widget(status_bar, status_area);
}

pub fn handle_key(app: &mut App, key: crossterm::event::KeyEvent) {
    use crossterm::event::{KeyCode, KeyEventKind};
    if key.kind != KeyEventKind::Press {
        return;
    }
    match key.code {
        KeyCode::Char('y') => app.do_sync(),
        KeyCode::Char('n') | KeyCode::Esc => {
            app.mode = crate::app::Mode::FileTree;
            app.dry_run_output.clear();
            app.sync_command.clear();
        }
        KeyCode::Char('?') => {
            app.previous_mode = Some(app.mode.clone());
            app.mode = crate::app::Mode::Help;
        }
        _ => {}
    }
}

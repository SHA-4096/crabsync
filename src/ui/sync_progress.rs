use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let (title, border_color) = if app.sync_error {
        (" Sync - FAILED ", Color::Red)
    } else {
        (" Sync - Complete ", Color::Green)
    };

    let title = if let Some(pair) = &app.current_pair {
        format!("{}{} ", title, pair.name)
    } else {
        title.to_string()
    };

    let content = if app.sync_output.is_empty() {
        "Sync completed with no output.".to_string()
    } else {
        app.sync_output.clone()
    };

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .style(Style::default().fg(border_color)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);

    let status = "Enter/Esc: return to pair list | ?: help";
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
        KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') => {
            app.current_pair = None;
            app.tree = None;
            app.tree_items.clear();
            app.tree_cursor = 0;
            app.dry_run_output.clear();
            app.sync_output.clear();
            app.sync_error = false;
            app.mode = crate::app::Mode::PairList;
            app.status_msg.clear();
            app.pairs = crate::config::load_all_pairs();
        }
        KeyCode::Char('?') => {
            app.previous_mode = Some(app.mode.clone());
            app.mode = crate::app::Mode::Help;
        }
        _ => {}
    }
}

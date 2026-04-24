mod file_tree;
mod pair_list;
mod sync_preview;
mod sync_progress;

use ratatui::layout::Rect;
use ratatui::Frame;

use crate::app::{App, Mode};

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();
    match app.mode {
        Mode::PairList => pair_list::draw(f, app, area),
        Mode::FileTree => file_tree::draw(f, app, area),
        Mode::SyncPreview => sync_preview::draw(f, app, area),
        Mode::SyncProgress => sync_progress::draw(f, app, area),
        Mode::Help => draw_help(f, app, area),
    }
}

pub fn handle_key(app: &mut App, key: crossterm::event::KeyEvent) {
    match app.mode {
        Mode::PairList => pair_list::handle_key(app, key),
        Mode::FileTree => file_tree::handle_key(app, key),
        Mode::SyncPreview => sync_preview::handle_key(app, key),
        Mode::SyncProgress => sync_progress::handle_key(app, key),
        Mode::Help => {
            if let crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Esc = key.code
            {
                app.mode = app.previous_mode.clone().unwrap_or(Mode::PairList);
            }
        }
    }
}

fn draw_help(f: &mut Frame, _app: &App, area: Rect) {
    use ratatui::layout::Constraint;
    use ratatui::style::{Color, Style};
    use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

    let text = vec![
        ratatui::text::Line::from("rusync - Key Bindings"),
        ratatui::text::Line::from(""),
        ratatui::text::Line::from("Pair List:"),
        ratatui::text::Line::from("  j/↓     - move down"),
        ratatui::text::Line::from("  k/↑     - move up"),
        ratatui::text::Line::from("  Enter   - enter file tree"),
        ratatui::text::Line::from("  d       - delete pair"),
        ratatui::text::Line::from(""),
        ratatui::text::Line::from("File Tree:"),
        ratatui::text::Line::from("  j/↓     - move down"),
        ratatui::text::Line::from("  k/↑     - move up"),
        ratatui::text::Line::from("  Space   - toggle selection"),
        ratatui::text::Line::from("  Enter   - expand/collapse dir"),
        ratatui::text::Line::from("  a       - select/deselect all"),
        ratatui::text::Line::from("  s       - sync selected (dry-run)"),
        ratatui::text::Line::from(""),
        ratatui::text::Line::from("Sync Preview:"),
        ratatui::text::Line::from("  y       - confirm sync"),
        ratatui::text::Line::from("  n/Esc   - go back"),
        ratatui::text::Line::from(""),
        ratatui::text::Line::from("Sync Progress:"),
        ratatui::text::Line::from("  Enter/Esc - return to pair list"),
        ratatui::text::Line::from(""),
        ratatui::text::Line::from("Global:"),
        ratatui::text::Line::from("  ?       - show help"),
        ratatui::text::Line::from("  q/Esc   - go back / quit"),
    ];

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

    let popup_area = ratatui::layout::Layout::default()
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(area)[1];

    let popup_area = ratatui::layout::Layout::default()
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(popup_area)[1];

    f.render_widget(Clear, popup_area);
    f.render_widget(paragraph, popup_area);
}

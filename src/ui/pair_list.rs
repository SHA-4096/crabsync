use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Row, Table, TableState};
use ratatui::Frame;

use crate::app::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec![
        Cell::from("Name").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Local").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Remote").style(Style::default().add_modifier(Modifier::BOLD)),
    ])
    .style(Style::default().fg(Color::Yellow))
    .bottom_margin(1);

    let rows: Vec<Row> = app
        .pairs
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let style = if i == app.pair_index {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(p.name.as_str()).style(style),
                Cell::from(p.local.as_str()).style(style),
                Cell::from(p.remote.as_str()).style(style),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            ratatui::layout::Constraint::Percentage(20),
            ratatui::layout::Constraint::Percentage(40),
            ratatui::layout::Constraint::Percentage(40),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(" Rusync - Pair List ")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Green)),
    );

    let mut state = TableState::default();
    state.select(Some(app.pair_index));
    f.render_stateful_widget(table, area, &mut state);

    let status = if app.status_msg.is_empty() {
        "Enter: select | d: delete | ?: help | q: quit".to_string()
    } else {
        app.status_msg.clone()
    };
    let status_bar =
        ratatui::widgets::Paragraph::new(status).style(Style::default().fg(Color::DarkGray));
    let status_area = Rect::new(area.x, area.y + area.height - 1, area.width, 1);
    f.render_widget(status_bar, status_area);
}

pub fn handle_key(app: &mut App, key: crossterm::event::KeyEvent) {
    use crossterm::event::{KeyCode, KeyEventKind};
    if key.kind != KeyEventKind::Press {
        return;
    }
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('?') => {
            app.previous_mode = Some(app.mode.clone());
            app.mode = crate::app::Mode::Help;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.pairs.is_empty() && app.pair_index < app.pairs.len() - 1 {
                app.pair_index += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.pair_index > 0 {
                app.pair_index -= 1;
            }
        }
        KeyCode::Enter => {
            if let Err(e) = app.enter_file_tree() {
                app.status_msg = format!("error: {}", e);
            }
        }
        KeyCode::Char('d') => {
            if !app.pairs.is_empty() {
                let name = app.pairs[app.pair_index].name.clone();
                if let Err(e) = crate::config::remove_pair(&name) {
                    app.status_msg = format!("error: {}", e);
                } else {
                    app.pairs = crate::config::load_pairs().unwrap_or_default();
                    if app.pair_index >= app.pairs.len() && app.pair_index > 0 {
                        app.pair_index -= 1;
                    }
                    app.status_msg = format!("deleted pair '{}'", name);
                }
            }
        }
        KeyCode::Esc => app.should_quit = true,
        _ => {}
    }
}

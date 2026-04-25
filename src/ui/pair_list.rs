use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Row, Table, TableState};
use ratatui::Frame;

use crate::app::App;
use crate::config::PairSource;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec![
        Cell::from("Name").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Local").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Remote").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Scope").style(Style::default().add_modifier(Modifier::BOLD)),
    ])
    .style(Style::default().fg(Color::Yellow))
    .bottom_margin(1);

    let mut selectable_indices: Vec<usize> = Vec::new();
    let mut rows: Vec<Row> = Vec::new();
    let mut in_local = false;
    let mut in_global = false;

    for (_i, tp) in app.pairs.iter().enumerate() {
        if tp.source == PairSource::Local && !in_local {
            in_local = true;
            rows.push(
                Row::new(vec![Cell::from("── Local ──").style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )])
                .style(Style::default()),
            );
        }
        if tp.source == PairSource::Global && !in_global {
            in_global = true;
            rows.push(
                Row::new(vec![Cell::from("── Global ──").style(
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                )])
                .style(Style::default()),
            );
        }

        let selected_idx = selectable_indices.len();
        let is_cursor = selected_idx == app.pair_index;
        selectable_indices.push(rows.len());

        let style = if is_cursor {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::REVERSED)
        } else if tp.shadowed {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default()
        };

        let scope_label = match tp.source {
            PairSource::Local => "local",
            PairSource::Global => "global",
        };
        let name_display = if tp.shadowed {
            format!("{} (shadowed)", tp.pair.name)
        } else {
            tp.pair.name.clone()
        };

        rows.push(Row::new(vec![
            Cell::from(name_display).style(style),
            Cell::from(tp.pair.local.as_str()).style(style),
            Cell::from(tp.pair.remote.as_str()).style(style),
            Cell::from(scope_label).style(style),
        ]));
    }

    let table = Table::new(
        rows,
        [
            ratatui::layout::Constraint::Percentage(25),
            ratatui::layout::Constraint::Percentage(30),
            ratatui::layout::Constraint::Percentage(30),
            ratatui::layout::Constraint::Percentage(15),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(" Crabsync - Pair List ")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Green)),
    );

    let mut state = TableState::default();
    if let Some(&row_idx) = selectable_indices.get(app.pair_index) {
        state.select(Some(row_idx));
    }
    f.render_stateful_widget(table, area, &mut state);

    let status = if app.status_msg.is_empty() {
        "Enter: select | a: add pair | d: delete | ?: help | q: quit".to_string()
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

    let num_selectable = app.pairs.len();

    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('?') => {
            app.previous_mode = Some(app.mode.clone());
            app.mode = crate::app::Mode::Help;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if num_selectable > 0 && app.pair_index < num_selectable - 1 {
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
            if let Some(tp) = app.pairs.get(app.pair_index).cloned() {
                let name = tp.pair.name.clone();
                let source = tp.source.clone();
                if let Err(e) = crate::config::remove_pair(&name, source) {
                    app.status_msg = format!("error: {}", e);
                } else {
                    app.refresh_pairs();
                    app.status_msg = format!(
                        "deleted pair '{}' from {} config",
                        name,
                        match source {
                            PairSource::Local => "local",
                            PairSource::Global => "global",
                        }
                    );
                }
            }
        }
        KeyCode::Char('a') => {
            app.start_add_pair();
        }
        KeyCode::Esc => app.should_quit = true,
        _ => {}
    }
}

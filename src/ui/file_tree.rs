use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let title = if let Some(pair) = &app.current_pair {
        format!(
            " File Tree - {} ({} -> {}) ",
            pair.name, pair.local, pair.remote
        )
    } else {
        " File Tree ".to_string()
    };

    let items: Vec<ListItem> = app
        .tree_items
        .iter()
        .enumerate()
        .map(|(i, (depth, node))| {
            let indent = "  ".repeat(*depth);
            let check = if node.is_dir {
                if node.expanded {
                    "▼ "
                } else {
                    "▶ "
                }
            } else if node.selected {
                "☑ "
            } else {
                "☐ "
            };

            let dir_check = if node.is_dir {
                if node.selected {
                    "☑ "
                } else {
                    "☐ "
                }
            } else {
                ""
            };

            let icon = if node.is_dir {
                if node.expanded {
                    "▼"
                } else {
                    "▶"
                }
            } else {
                ""
            };

            let name = node.display_name();
            let style = if i == app.tree_cursor {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::REVERSED)
            } else if node.selected {
                Style::default().fg(Color::Green)
            } else if node.is_dir {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };

            let line = if node.is_dir {
                Line::from(vec![
                    Span::styled(indent.clone(), style),
                    Span::styled(icon.to_string(), style),
                    Span::styled(" ".to_string(), style),
                    Span::styled(dir_check.to_string(), style),
                    Span::styled(name, style),
                ])
            } else {
                Line::from(vec![
                    Span::styled(indent.clone(), style),
                    Span::styled(check.to_string(), style),
                    Span::styled(name, style),
                ])
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Green)),
    );

    let mut state = ListState::default();
    state.select(Some(app.tree_cursor));
    f.render_stateful_widget(list, area, &mut state);

    let selected_count = app
        .tree
        .as_ref()
        .map(|t| t.collect_selected().len())
        .unwrap_or(0);
    let status = if app.status_msg.is_empty() {
        format!(
            "Space: toggle | Enter: expand | s: sync({} selected) | a: select all | ?: help | Esc: back",
            selected_count
        )
    } else {
        app.status_msg.clone()
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
        KeyCode::Char('q') | KeyCode::Esc => {
            app.current_pair = None;
            app.tree = None;
            app.tree_items.clear();
            app.tree_cursor = 0;
            app.mode = crate::app::Mode::PairList;
            app.status_msg.clear();
        }
        KeyCode::Char('?') => {
            app.previous_mode = Some(app.mode.clone());
            app.mode = crate::app::Mode::Help;
        }
        KeyCode::Char('j') | KeyCode::Down => app.tree_cursor_down(),
        KeyCode::Char('k') | KeyCode::Up => app.tree_cursor_up(),
        KeyCode::Char(' ') => app.toggle_tree_item(),
        KeyCode::Enter => app.toggle_expand(),
        KeyCode::Char('a') => app.toggle_select_all(),
        KeyCode::Char('s') => app.do_dry_run(),
        _ => {}
    }
}

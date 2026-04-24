use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::{ActivePanel, App, RemoteStatus};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let pair = app.current_pair.as_ref();
    let source_title = pair
        .map(|p| format!(" Source: {} ", p.local))
        .unwrap_or_else(|| " Source ".to_string());
    let target_title = pair
        .map(|p| format!(" Target: {} ", p.remote))
        .unwrap_or_else(|| " Target ".to_string());

    let panes = Layout::default()
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let source_active = app.active_panel == ActivePanel::Source;
    let source_style = if source_active {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let source_items = build_source_items(app);
    let source_list = List::new(source_items).block(
        Block::default()
            .title(source_title)
            .borders(Borders::ALL)
            .style(source_style),
    );

    let mut source_state = ListState::default();
    if source_active {
        source_state.select(Some(app.tree_cursor));
    } else {
        source_state.select(None);
    }
    f.render_stateful_widget(source_list, panes[0], &mut source_state);

    let target_active = app.active_panel == ActivePanel::Target;
    let target_style = if target_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    match &app.remote_status {
        RemoteStatus::Loaded => {
            let target_items = build_target_items(app);
            let target_list = List::new(target_items).block(
                Block::default()
                    .title(target_title)
                    .borders(Borders::ALL)
                    .style(target_style),
            );

            let mut target_state = ListState::default();
            if target_active {
                target_state.select(Some(app.remote_tree_cursor));
            } else {
                target_state.select(None);
            }
            f.render_stateful_widget(target_list, panes[1], &mut target_state);
        }
        RemoteStatus::AuthRequired => {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Authentication required",
                    Style::default().fg(Color::Yellow),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Press p to enter password",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(Span::styled(
                    "  or configure SSH key auth",
                    Style::default().fg(Color::DarkGray),
                )),
            ];
            let paragraph = Paragraph::new(lines).block(
                Block::default()
                    .title(target_title)
                    .borders(Borders::ALL)
                    .style(target_style),
            );
            f.render_widget(paragraph, panes[1]);
        }
        RemoteStatus::Error(msg) => {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Error loading remote",
                    Style::default().fg(Color::Red),
                )),
                Line::from(""),
            ];
            let mut err_lines: Vec<Line> = lines;
            for line in msg.lines().take(10) {
                err_lines.push(Line::from(Span::styled(
                    format!("  {}", line),
                    Style::default().fg(Color::DarkGray),
                )));
            }
            err_lines.push(Line::from(""));
            err_lines.push(Line::from(Span::styled(
                "  Press r to retry",
                Style::default().fg(Color::DarkGray),
            )));
            let paragraph = Paragraph::new(err_lines).block(
                Block::default()
                    .title(target_title)
                    .borders(Borders::ALL)
                    .style(target_style),
            );
            f.render_widget(paragraph, panes[1]);
        }
        RemoteStatus::Loading => {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Loading remote tree...",
                    Style::default().fg(Color::Yellow),
                )),
            ];
            let paragraph = Paragraph::new(lines).block(
                Block::default()
                    .title(target_title)
                    .borders(Borders::ALL)
                    .style(target_style),
            );
            f.render_widget(paragraph, panes[1]);
        }
        RemoteStatus::NotLoaded => {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Press r to load remote tree",
                    Style::default().fg(Color::DarkGray),
                )),
            ];
            let paragraph = Paragraph::new(lines).block(
                Block::default()
                    .title(target_title)
                    .borders(Borders::ALL)
                    .style(target_style),
            );
            f.render_widget(paragraph, panes[1]);
        }
    }

    let selected_count = app
        .tree
        .as_ref()
        .map(|t| t.collect_selected().len())
        .unwrap_or(0);
    let status = if app.status_msg.is_empty() {
        format!(
            "Space: toggle | Enter: expand | s: sync({} selected) | a: select all | Tab: switch panel | r: reload remote | ?: help | Esc: back",
            selected_count
        )
    } else {
        app.status_msg.clone()
    };
    let status_bar = Paragraph::new(status).style(Style::default().fg(Color::DarkGray));
    let status_area = Rect::new(area.x, area.y + area.height - 1, area.width, 1);
    f.render_widget(status_bar, status_area);
}

fn build_source_items<'a>(app: &App) -> Vec<ListItem<'a>> {
    app.tree_items
        .iter()
        .enumerate()
        .map(|(i, (depth, node))| {
            let indent = "  ".repeat(*depth);
            let style = if app.active_panel == ActivePanel::Source && i == app.tree_cursor {
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
                let icon = if node.expanded { "▼" } else { "▶" };
                let check = if node.selected { "☑" } else { "☐" };
                Line::from(vec![
                    Span::styled(indent.clone(), style),
                    Span::styled(icon.to_string(), style),
                    Span::styled(" ".to_string(), style),
                    Span::styled(check.to_string(), style),
                    Span::styled(" ".to_string(), style),
                    Span::styled(node.display_name(), style),
                ])
            } else {
                let check = if node.selected { "☑" } else { "☐" };
                Line::from(vec![
                    Span::styled(indent.clone(), style),
                    Span::styled(check.to_string(), style),
                    Span::styled(" ".to_string(), style),
                    Span::styled(node.display_name(), style),
                ])
            };

            ListItem::new(line).style(style)
        })
        .collect()
}

fn build_target_items<'a>(app: &App) -> Vec<ListItem<'a>> {
    app.remote_tree_items
        .iter()
        .enumerate()
        .map(|(i, (depth, node))| {
            let indent = "  ".repeat(*depth);
            let style = if app.active_panel == ActivePanel::Target && i == app.remote_tree_cursor {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::REVERSED)
            } else if node.is_dir {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };

            let line = if node.is_dir {
                let icon = if node.expanded { "▼" } else { "▶" };
                Line::from(vec![
                    Span::styled(indent.clone(), style),
                    Span::styled(icon.to_string(), style),
                    Span::styled(" ".to_string(), style),
                    Span::styled(node.display_name(), style),
                ])
            } else {
                Line::from(vec![
                    Span::styled(indent.clone(), style),
                    Span::styled(node.display_name(), style),
                ])
            };

            ListItem::new(line).style(style)
        })
        .collect()
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
            app.remote_tree = None;
            app.remote_tree_items.clear();
            app.remote_tree_cursor = 0;
            app.remote_status = RemoteStatus::NotLoaded;
            app.mode = crate::app::Mode::PairList;
            app.status_msg.clear();
        }
        KeyCode::Char('?') => {
            app.previous_mode = Some(app.mode.clone());
            app.mode = crate::app::Mode::Help;
        }
        KeyCode::Tab => app.toggle_panel(),
        KeyCode::Char('j') | KeyCode::Down => match app.active_panel {
            ActivePanel::Source => app.tree_cursor_down(),
            ActivePanel::Target => app.remote_tree_cursor_down(),
        },
        KeyCode::Char('k') | KeyCode::Up => match app.active_panel {
            ActivePanel::Source => app.tree_cursor_up(),
            ActivePanel::Target => app.remote_tree_cursor_up(),
        },
        KeyCode::Char(' ') => {
            if app.active_panel == ActivePanel::Source {
                app.toggle_tree_item();
            }
        }
        KeyCode::Enter => {
            if app.active_panel == ActivePanel::Source {
                app.toggle_expand();
            } else {
                app.toggle_expand_remote();
            }
        }
        KeyCode::Char('a') => {
            if app.active_panel == ActivePanel::Source {
                app.toggle_select_all();
            }
        }
        KeyCode::Char('s') => {
            if app.active_panel == ActivePanel::Source {
                app.do_dry_run();
            }
        }
        KeyCode::Char('r') => app.load_remote_tree(),
        KeyCode::Char('p') => {
            if matches!(app.remote_status, RemoteStatus::AuthRequired) {
                app.load_remote_interactive();
            }
        }
        _ => {}
    }
}

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::config::PairSource;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = Layout::default()
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ])
        .split(area)[1];

    let popup_area = Layout::default()
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ])
        .split(popup_area)[1];

    let fields = [
        ("Name:", &app.add_pair_name),
        ("Local:", &app.add_pair_local),
        ("Remote:", &app.add_pair_remote),
    ];

    let _scope_label = match app.add_pair_scope {
        PairSource::Local => "Local",
        PairSource::Global => "Global",
    };

    let mut lines: Vec<Line> = Vec::new();

    for (i, (label, value)) in fields.iter().enumerate() {
        let is_focused = app.add_pair_focus == i;
        let label_style = if is_focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let value_style = if is_focused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(Color::White)
        };

        let display_value = if value.is_empty() && is_focused {
            "_".to_string()
        } else if is_focused {
            format!("{}_", value)
        } else if value.is_empty() {
            " ".to_string()
        } else {
            value.to_string()
        };

        lines.push(Line::from(vec![
            Span::styled(format!("  {:<8}", label), label_style),
            Span::styled(display_value, value_style),
        ]));
        lines.push(Line::from(""));
    }

    let is_scope_focused = app.add_pair_focus == 3;
    let scope_style = if is_scope_focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    let local_style = if app.add_pair_scope == PairSource::Local {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let global_style = if app.add_pair_scope == PairSource::Global {
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    lines.push(Line::from(vec![
        Span::styled("  Scope:   ", scope_style),
        Span::styled("[Local]", local_style),
        Span::styled(" / ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Global]", global_style),
    ]));

    if !app.status_msg.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", app.status_msg),
            Style::default().fg(Color::Red),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Tab: next field | Space: toggle scope | Enter: save | Esc: cancel",
        Style::default().fg(Color::DarkGray),
    )));

    let block = Block::default()
        .title(" Add Pair ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(Clear, popup_area);
    f.render_widget(paragraph, popup_area);
}

pub fn handle_key(app: &mut App, key: crossterm::event::KeyEvent) {
    use crossterm::event::{KeyCode, KeyEventKind};
    if key.kind != KeyEventKind::Press {
        return;
    }

    match key.code {
        KeyCode::Esc => {
            app.mode = crate::app::Mode::PairList;
            app.status_msg.clear();
        }
        KeyCode::Tab => {
            app.add_pair_focus = (app.add_pair_focus + 1) % 4;
        }
        KeyCode::BackTab => {
            app.add_pair_focus = (app.add_pair_focus + 3) % 4;
        }
        KeyCode::Enter => {
            app.add_pair_from_form();
        }
        KeyCode::Char(' ') if app.add_pair_focus == 3 => {
            app.add_pair_scope = match app.add_pair_scope {
                PairSource::Local => PairSource::Global,
                PairSource::Global => PairSource::Local,
            };
        }
        KeyCode::Char(c) => match app.add_pair_focus {
            0 => app.add_pair_name.push(c),
            1 => app.add_pair_local.push(c),
            2 => app.add_pair_remote.push(c),
            _ => {}
        },
        KeyCode::Backspace => match app.add_pair_focus {
            0 => {
                app.add_pair_name.pop();
            }
            1 => {
                app.add_pair_local.pop();
            }
            2 => {
                app.add_pair_remote.pop();
            }
            _ => {}
        },
        KeyCode::Delete => match app.add_pair_focus {
            0 => {
                app.add_pair_name.clear();
            }
            1 => {
                app.add_pair_local.clear();
            }
            2 => {
                app.add_pair_remote.clear();
            }
            _ => {}
        },
        _ => {}
    }
}

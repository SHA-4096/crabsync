mod app;
mod cli;
mod config;
mod sync;
mod tree;
mod ui;

use anyhow::Result;
use clap::Parser;
use crossterm::event::{self, Event};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    if let Some(cmd) = &cli.command {
        if matches!(cmd, cli::Commands::Sync { .. }) {
            let name = match cmd {
                cli::Commands::Sync { name } => name.clone(),
                _ => unreachable!(),
            };
            return run_tui(Some(&name));
        }
        cli::handle_command(cmd)?;
        return Ok(());
    }

    run_tui(None)
}

fn run_tui(initial_pair: Option<&str>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = app::App::new(initial_pair)?;
    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut app::App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                ui::handle_key(app, key);
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

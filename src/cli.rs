use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::config;

#[derive(Parser)]
#[command(name = "crabsync", about = "A TUI for rsync file synchronization")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(long, short, default_value = ".")]
    pub project_root: PathBuf,
}

#[derive(Subcommand)]
pub enum Commands {
    Add {
        name: String,
        local: String,
        remote: String,
        #[arg(long, default_value_t = false)]
        global: bool,
    },
    Remove {
        name: String,
        #[arg(long, default_value_t = false)]
        global: bool,
    },
    List,
    Sync {
        name: String,
    },
}

pub fn handle_command(cmd: &Commands) -> anyhow::Result<()> {
    match cmd {
        Commands::Add {
            name,
            local,
            remote,
            global,
        } => {
            let source = if *global {
                config::PairSource::Global
            } else {
                config::PairSource::Local
            };
            config::add_pair(name.clone(), local.clone(), remote.clone(), source)?;
            let scope = if *global { "global" } else { "local" };
            println!(
                "added pair '{}' to {} config: {} -> {}",
                name, scope, local, remote
            );
            Ok(())
        }
        Commands::Remove { name, global } => {
            if *global {
                config::remove_pair(name, config::PairSource::Global)?;
                println!("removed pair '{}' from global config", name);
            } else {
                match config::remove_pair(name, config::PairSource::Local) {
                    Ok(()) => {
                        println!("removed pair '{}' from local config", name);
                    }
                    Err(_) => {
                        config::remove_pair(name, config::PairSource::Global)?;
                        println!("removed pair '{}' from global config", name);
                    }
                }
            }
            Ok(())
        }
        Commands::List => {
            let all = config::load_all_pairs();
            if all.is_empty() {
                println!("no pairs configured");
            } else {
                for tp in &all {
                    let scope = match tp.source {
                        config::PairSource::Local => "local",
                        config::PairSource::Global => "global",
                    };
                    let shadowed = if tp.shadowed { " (shadowed)" } else { "" };
                    println!(
                        "{}: {} -> {} [{}]{}",
                        tp.pair.name, tp.pair.local, tp.pair.remote, scope, shadowed
                    );
                }
            }
            Ok(())
        }
        Commands::Sync { name: _ } => Ok(()),
    }
}

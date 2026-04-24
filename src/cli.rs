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
    },
    Remove {
        name: String,
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
        } => {
            config::add_pair(name.clone(), local.clone(), remote.clone())?;
            println!("added pair '{}': {} -> {}", name, local, remote);
            Ok(())
        }
        Commands::Remove { name } => {
            config::remove_pair(name)?;
            println!("removed pair '{}'", name);
            Ok(())
        }
        Commands::List => {
            let pairs = config::load_pairs()?;
            if pairs.is_empty() {
                println!("no pairs configured");
            } else {
                for p in &pairs {
                    println!("{}: {} -> {}", p.name, p.local, p.remote);
                }
            }
            Ok(())
        }
        Commands::Sync { name: _ } => Ok(()),
    }
}

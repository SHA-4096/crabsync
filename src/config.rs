use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub static CONFIG_DIR: &str = "config/rusync";
pub static PAIRS_FILE: &str = "config/rusync/pairs.toml";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Pair {
    pub name: String,
    pub local: String,
    pub remote: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct PairsConfig {
    pub pairs: Vec<Pair>,
}

pub fn config_dir() -> PathBuf {
    PathBuf::from(CONFIG_DIR)
}

pub fn pairs_file() -> PathBuf {
    PathBuf::from(PAIRS_FILE)
}

pub fn load_pairs() -> Result<Vec<Pair>> {
    let path = pairs_file();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let config: PairsConfig =
        toml::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(config.pairs)
}

pub fn save_pairs(pairs: &[Pair]) -> Result<()> {
    let dir = config_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    }
    let config = PairsConfig {
        pairs: pairs.to_vec(),
    };
    let content = toml::to_string_pretty(&config).context("failed to serialize pairs")?;
    let path = pairs_file();
    fs::write(&path, content).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

pub fn add_pair(name: String, local: String, remote: String) -> Result<()> {
    let mut pairs = load_pairs()?;
    if pairs.iter().any(|p| p.name == name) {
        anyhow::bail!("pair '{}' already exists", name);
    }
    pairs.push(Pair {
        name,
        local,
        remote,
    });
    save_pairs(&pairs)
}

pub fn remove_pair(name: &str) -> Result<()> {
    let mut pairs = load_pairs()?;
    let before = pairs.len();
    pairs.retain(|p| p.name != name);
    if pairs.len() == before {
        anyhow::bail!("pair '{}' not found", name);
    }
    save_pairs(&pairs)
}

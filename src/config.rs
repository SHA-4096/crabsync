use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum PairSource {
    Global,
    Local,
}

#[derive(Debug, Clone)]
pub struct TaggedPair {
    pub pair: Pair,
    pub source: PairSource,
    pub shadowed: bool,
}

pub fn global_config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".config/crabsync/crabsync.toml")
}

pub fn local_config_path() -> PathBuf {
    PathBuf::from("crabsync.toml")
}

fn load_pairs_from(path: &PathBuf) -> Result<Vec<Pair>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let config: PairsConfig =
        toml::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(config.pairs)
}

fn save_pairs_to(pairs: &[Pair], path: &PathBuf) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
    }
    let config = PairsConfig {
        pairs: pairs.to_vec(),
    };
    let content = toml::to_string_pretty(&config).context("failed to serialize pairs")?;
    fs::write(path, content).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

pub fn load_global_pairs() -> Result<Vec<Pair>> {
    load_pairs_from(&global_config_path())
}

pub fn load_local_pairs() -> Result<Vec<Pair>> {
    load_pairs_from(&local_config_path())
}

pub fn save_global_pairs(pairs: &[Pair]) -> Result<()> {
    save_pairs_to(pairs, &global_config_path())
}

pub fn save_local_pairs(pairs: &[Pair]) -> Result<()> {
    save_pairs_to(pairs, &local_config_path())
}

pub fn load_all_pairs() -> Vec<TaggedPair> {
    let local_pairs = load_local_pairs().unwrap_or_default();
    let global_pairs = load_global_pairs().unwrap_or_default();

    let local_names: Vec<String> = local_pairs.iter().map(|p| p.name.clone()).collect();

    let mut result: Vec<TaggedPair> = local_pairs
        .into_iter()
        .map(|p| TaggedPair {
            pair: p,
            source: PairSource::Local,
            shadowed: false,
        })
        .collect();

    for p in global_pairs {
        let shadowed = local_names.contains(&p.name);
        result.push(TaggedPair {
            pair: p,
            source: PairSource::Global,
            shadowed,
        });
    }

    result
}

#[allow(dead_code)]
pub fn find_pair_by_name(name: &str) -> Option<TaggedPair> {
    let all = load_all_pairs();
    all.into_iter()
        .find(|tp| tp.pair.name == name && !tp.shadowed)
        .or_else(|| load_all_pairs().into_iter().find(|tp| tp.pair.name == name))
}

pub fn add_pair(name: String, local: String, remote: String, source: PairSource) -> Result<()> {
    match source {
        PairSource::Local => {
            let mut pairs = load_local_pairs()?;
            if pairs.iter().any(|p| p.name == name) {
                anyhow::bail!("pair '{}' already exists in local config", name);
            }
            pairs.push(Pair {
                name,
                local,
                remote,
            });
            save_local_pairs(&pairs)
        }
        PairSource::Global => {
            let mut pairs = load_global_pairs()?;
            if pairs.iter().any(|p| p.name == name) {
                anyhow::bail!("pair '{}' already exists in global config", name);
            }
            pairs.push(Pair {
                name,
                local,
                remote,
            });
            save_global_pairs(&pairs)
        }
    }
}

pub fn remove_pair(name: &str, source: PairSource) -> Result<()> {
    match source {
        PairSource::Local => {
            let mut pairs = load_local_pairs()?;
            let before = pairs.len();
            pairs.retain(|p| p.name != name);
            if pairs.len() == before {
                anyhow::bail!("pair '{}' not found in local config", name);
            }
            if pairs.is_empty() {
                let path = local_config_path();
                if path.exists() {
                    fs::remove_file(&path)
                        .with_context(|| format!("failed to remove {}", path.display()))?;
                }
            } else {
                save_local_pairs(&pairs)?;
            }
            Ok(())
        }
        PairSource::Global => {
            let mut pairs = load_global_pairs()?;
            let before = pairs.len();
            pairs.retain(|p| p.name != name);
            if pairs.len() == before {
                anyhow::bail!("pair '{}' not found in global config", name);
            }
            if pairs.is_empty() {
                let path = global_config_path();
                if path.exists() {
                    fs::remove_file(&path)
                        .with_context(|| format!("failed to remove {}", path.display()))?;
                }
            } else {
                save_global_pairs(&pairs)?;
            }
            Ok(())
        }
    }
}

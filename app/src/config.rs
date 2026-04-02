use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result};
use dirs::home_dir;
use serde::Deserialize;

#[derive(Clone, Debug)]
pub struct Config {
    pub base_url: String,
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    base_url: Option<String>,
}

pub fn resolve(base_url: Option<String>) -> Result<Config> {
    let file_path = env::var("SANTI_CLI_CONFIG_FILE")
        .ok()
        .or_else(default_config_file)
        .context("could not determine config file path")?;
    let file_config = read_file_config(PathBuf::from(file_path))?;

    let base_url = base_url
        .or_else(|| env::var("SANTI_CLI_BASE_URL").ok())
        .or(file_config.base_url)
        .unwrap_or_else(|| "http://127.0.0.1:18081".to_string());

    Ok(Config { base_url })
}

fn default_config_file() -> Option<String> {
    home_dir().map(|path| {
        path.join(".santi-cli")
            .join("config.json")
            .display()
            .to_string()
    })
}

fn read_file_config(path: PathBuf) -> Result<FileConfig> {
    if !path.exists() {
        return Ok(FileConfig::default());
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("read config file failed ({})", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("parse config file failed ({})", path.display()))
}

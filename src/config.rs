use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
struct ConfigFile {
    pub msg_template: Option<String>,
    pub template: Option<String>,
    pub bar_chars: Option<String>,
    pub workers: Option<usize>,
}

#[derive(Debug)]
pub struct Config {
    pub msg_template: String,
    pub template: String,
    pub bar_chars: String,
    pub workers: usize,
}

impl Config {
    pub fn load(path: &str) -> Self {
        let config_file: ConfigFile = fs::read_to_string(path)
            .ok()
            .and_then(|content| toml::from_str(&content).ok())
            .unwrap_or(ConfigFile {
                msg_template: None,
                template: None,
                bar_chars: None,
                workers: None,
            });
        let default = Self::default();
        Self {
            msg_template: config_file.msg_template.unwrap_or(default.msg_template),
            template: config_file.template.unwrap_or(default.template),
            bar_chars: config_file.bar_chars.unwrap_or(default.bar_chars),
            workers: config_file.workers.unwrap_or(default.workers),
        }
    }

    pub fn load_from_config_dir() -> Self {
        if let Some(mut path) = dirs::config_dir() {
            path.push("dwrs");
            path.push("config.toml");
            return Self::load(path.to_str().unwrap_or_default());
        }
        log::warn!("Config dir not found, using default config");
        Self::default()
    }

    pub fn default() -> Self {
        Self {
            msg_template: "{download} {url} → {output}".to_string(),
            template: "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} ({percent}%) {msg}".to_string(),
            bar_chars: "█▌░".to_string(),
            workers: 1,
        }
    }
}

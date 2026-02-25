use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
struct ConfigFile {
    pub msg_template: Option<String>,
    pub template: Option<String>,
    pub bar_chars: Option<String>,
    pub workers: Option<usize>,
    pub buffer_size: Option<usize>,
    pub pool_size: Option<usize>,
    pub retries: Option<usize>,
    pub min_parallel_size: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub msg_template: String,
    pub template: String,
    pub bar_chars: String,
    pub workers: usize,
    pub buffer_size: usize,
    pub pool_size: usize,
    pub retries: usize,
    pub min_parallel_size: u64,
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
                buffer_size: None,
                pool_size: None,
                retries: None,
                min_parallel_size: None,
            });
        let default = Self::default();
        Self {
            msg_template: config_file.msg_template.unwrap_or(default.msg_template),
            template: config_file.template.unwrap_or(default.template),
            bar_chars: config_file.bar_chars.unwrap_or(default.bar_chars),
            workers: config_file.workers.unwrap_or(default.workers),
            buffer_size: config_file.buffer_size.unwrap_or(default.buffer_size),
            pool_size: config_file.pool_size.unwrap_or(default.pool_size),
            retries: config_file.retries.unwrap_or(default.retries),
            min_parallel_size: config_file
                .min_parallel_size
                .unwrap_or(default.min_parallel_size),
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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            msg_template: "{download} {url} → {output}".to_string(),
            template: "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} ({percent}%) {msg}".to_string(),
            bar_chars: "█▌░".to_string(),
            workers: 4,
            buffer_size: 256 * 1024,
            pool_size: 100,
            retries: 3,
            min_parallel_size: 5 * 1024 * 1024,
        }
    }
}

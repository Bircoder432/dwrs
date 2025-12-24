#[macro_use]
extern crate rust_i18n;

i18n!("locale", fallback = "en");

use clap::Parser;
use colored::Colorize;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use log::{error, info};
use std::path::PathBuf;

use dwrs::cli::Args;
use dwrs::config::Config;
use dwrs::{Downloader, init, notify_send};

#[tokio::main]
async fn main() {
    init();
    info!("Logger initialized");

    let args = Args::parse();
    let mut cfg = Config::load_from_config_dir();
    if args.config.is_some() {
        cfg = Config::load(&args.config.unwrap());
    }

    let mut workers = cfg.workers;
    if args.workers != 1 {
        workers = args.workers as u8;
    }
    let download_config = dwrs::DownloadConfig {
        workers: workers as usize,
        template: cfg.template,
        chars: cfg.bar_chars,
        continue_download: args.continue_,
        notify: args.notify,
    };

    let downloader = Downloader::new(download_config);

    if args.background {
        if let Err(e) = dwrs::spawn_background_process() {
            error!("Failed to spawn background process: {}", e);
        }
        return;
    }

    let downloads: Vec<(String, PathBuf)> = if let Some(file_path) = args.file {
        match dwrs::parse_file(&file_path).await {
            Ok(pairs) => pairs
                .into_iter()
                .map(|(url, path)| (url, PathBuf::from(path)))
                .collect(),
            Err(e) => {
                eprintln!("{}: {}", t!("error-in-reading-file").red().bold(), e);
                return;
            }
        }
    } else {
        let mut pairs = Vec::new();
        for (i, url) in args.url.iter().enumerate() {
            let output = if let Some(path) = args.output.get(i) {
                PathBuf::from(path)
            } else {
                PathBuf::from(url.split('/').last().unwrap_or("file.bin"))
            };
            pairs.push((url.clone(), output));
        }
        if !args.output.is_empty() && args.output.len() != args.url.len() {
            error!("Error: number of output files does not match number of URLs");
            eprintln!("{}", t!("error-count").red().bold());
            return;
        }
        pairs
    };

    let downloads_refs: Vec<(&str, PathBuf)> = downloads
        .iter()
        .map(|(url, path)| (url.as_str(), path.clone()))
        .collect();

    if let Err(e) = downloader.download_multiple(downloads_refs).await {
        error!("Error during downloads: {}", e);
    }
}

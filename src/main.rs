use clap::Parser;
use colored::Colorize;
use dwrs::cli::Args;
use dwrs::config::Config;
use dwrs::{Downloader, init};
use log::{error, info};
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    init();
    info!("Logger initialized");

    let args = Args::parse();
    let mut cfg = Config::load_from_config_dir();

    if let Some(config_path) = args.config {
        cfg = Config::load(&config_path);
    }

    let workers = if args.workers != 4 {
        args.workers
    } else {
        cfg.workers
    };
    let buffer_size = args
        .buffer_size
        .map(|kb| kb * 1024)
        .unwrap_or(cfg.buffer_size);
    let pool_size = if args.pool_size != 100 {
        args.pool_size
    } else {
        cfg.pool_size
    };
    let retries = if args.retries != 3 {
        args.retries
    } else {
        cfg.retries
    };
    let min_parallel_size = if args.min_parallel_size != 5 {
        args.min_parallel_size * 1024 * 1024
    } else {
        cfg.min_parallel_size
    };

    let download_config = dwrs::DownloadConfig {
        workers,
        msg_template: cfg.msg_template,
        template: cfg.template,
        chars: cfg.bar_chars,
        continue_download: args.continue_,
        #[cfg(feature = "notify")]
        notify: args.notify,
        buffer_size,
        pool_size,
        retries,
        min_parallel_size,
        max_concurrent_files: args.max_files,
    };

    let downloader = Downloader::new(download_config);

    if args.background {
        #[cfg(feature = "notify")]
        if let Err(e) = dwrs::spawn_background_process() {
            error!("Failed to spawn background process: {}", e);
        }
        #[cfg(not(feature = "notify"))]
        error!("Background mode requires 'notify' feature");
        return;
    }

    let downloads: Vec<(String, PathBuf)> = if let Some(file_path) = args.file {
        match dwrs::parse_file(&file_path).await {
            Ok(pairs) => pairs
                .into_iter()
                .map(|(url, path)| (url, PathBuf::from(path)))
                .collect(),
            Err(e) => {
                eprintln!("{}: {}", "Error reading file".red().bold(), e);
                std::process::exit(1);
            }
        }
    } else {
        let mut pairs = Vec::new();
        for (i, url) in args.url.iter().enumerate() {
            let output = if let Some(path) = args.output.get(i) {
                PathBuf::from(path)
            } else {
                PathBuf::from(url.split('/').next_back().unwrap_or("file.bin"))
            };
            pairs.push((url.clone(), output));
        }

        if !args.output.is_empty() && args.output.len() != args.url.len() {
            error!("Error: number of output files does not match number of URLs");
            eprintln!("{}", "Error: count mismatch".red().bold());
            std::process::exit(1);
        }
        pairs
    };

    if downloads.is_empty() {
        eprintln!("{}", "No downloads to process".red().bold());
        std::process::exit(1);
    }

    info!("Starting {} download(s)", downloads.len());

    let downloads_refs: Vec<(&str, PathBuf)> = downloads
        .iter()
        .map(|(url, path)| (url.as_str(), path.clone()))
        .collect();

    match downloader.download_multiple(downloads_refs).await {
        Ok(_) => {
            info!("All downloads completed successfully");
        }
        Err(e) => {
            error!("Error during downloads: {}", e);
            std::process::exit(1);
        }
    }
}

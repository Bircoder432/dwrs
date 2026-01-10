//! A powerful and flexible file downloader library with parallel downloads,
//! resumable downloads, progress tracking, and desktop notifications.
//!
//! # Features
//!
//! - Parallel downloads with configurable concurrency
//! - Resumable downloads (continue interrupted downloads)
//! - Progress tracking with visual progress bars
//! - Desktop notifications on completion/failure
//! - Internationalization (i18n) support
//! - File-based batch downloads
//! - Background process support

#[macro_use]
extern crate rust_i18n;

i18n!("i18n", fallback = "en");

use std::path::PathBuf;
use std::sync::Arc;
pub mod cli;
pub mod config;
pub mod download;
pub mod file_parser;
pub mod notifications;
pub mod progress;
pub mod utils;

use futures::stream::{FuturesUnordered, StreamExt};
use indicatif::MultiProgress;
use notify_rust::Notification;
use reqwest::Client;
use rust_i18n::t;
use tokio::sync::Semaphore;
use tokio::task;

pub use download::download_file;
pub use file_parser::parse_file;

/// Configuration for the downloader
#[derive(Debug, Clone)]
pub struct DownloadConfig {
    /// Number of parallel downloads
    pub workers: usize,
    /// Whether to resume interrupted downloads
    pub continue_download: bool,
    /// Whether to show desktop notifications
    pub notify: bool,
    /// Template for progressbar
    pub template: String,
    /// template for msg
    pub msg_template: String,
    /// Chars for progress bar "FPE" Full Partial Empty
    pub chars: String,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            workers: 1,
            template: "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} ({percent}%) {msg}".to_string(),
            msg_template: "{download} {url} → {output}".to_string(),
            chars: "█▌░".to_string(),
            continue_download: false,
            notify: false,
        }
    }
}

/// Main downloader struct that manages parallel downloads
pub struct Downloader {
    config: DownloadConfig,
    client: Client,
}

impl Downloader {
    /// Create a new Downloader with the given configuration
    pub fn new(config: DownloadConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    /// Create a new Downloader with default configuration
    pub fn new_default() -> Self {
        Self::new(DownloadConfig::default())
    }

    /// Download a single file
    ///
    /// # Arguments
    ///
    /// * `url` - The URL of the file to download
    /// * `output_path` - The path where the file should be saved
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub async fn download_file(
        &self,
        url: &str,
        output_path: PathBuf,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mp = Arc::new(MultiProgress::new());
        let pb = progress::create_progress_bar(
            &mp,
            &self.config.msg_template,
            &self.config.template,
            &self.config.chars,
            url,
            output_path.to_str().unwrap_or("file"),
        );

        download::download_file(
            &self.client,
            url,
            &output_path,
            &pb,
            self.config.continue_download,
            self.config.workers,
        )
        .await?;
        if self.config.notify {
            Notification::new()
                .summary("Download end")
                .body("Download end")
                .show()
                .ok();
        }
        Ok(())
    }

    /// Download multiple files in parallel
    ///
    /// # Arguments
    ///
    /// * `downloads` - Vector of (URL, output_path) pairs to download
    ///
    /// # Returns
    ///
    /// Result indicating overall success or failure
    pub async fn download_multiple(
        &self,
        downloads: Vec<(&str, PathBuf)>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mp = Arc::new(MultiProgress::new());
        let semaphore = Arc::new(Semaphore::new(self.config.workers));
        let mut tasks = FuturesUnordered::new();

        for (url, output_path) in downloads.into_iter() {
            let client = self.client.clone();
            let mp = mp.clone();
            let sem = semaphore.clone();
            let output_str = output_path.to_string_lossy().to_string();
            let resume = self.config.continue_download;
            let url = url.to_string();
            let jobs = self.config.workers;
            let template = self.config.template.clone();
            let msg_template = self.config.msg_template.clone();
            let chars = self.config.chars.clone();

            tasks.push(task::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                let pb = progress::create_progress_bar(
                    &mp,
                    &template,
                    &msg_template,
                    &chars,
                    &url,
                    &output_str,
                );

                match download_file(&client, &url, &output_path, &pb, resume, jobs).await {
                    Ok(_) => {
                        pb.finish_with_message(format!(
                            "{}: {}",
                            t!("download-finish"),
                            output_str
                        ));
                        Ok(())
                    }
                    Err(e) => {
                        pb.finish_with_message(format!(
                            "{}: {}: {}",
                            t!("download-error"),
                            output_str,
                            e
                        ));
                        Err(e)
                    }
                }
            }));
        }

        while let Some(result) = tasks.next().await {
            let _ = result??;
        }
        if self.config.notify {
            Notification::new()
                .summary("Downloading end")
                .body("download end")
                .show()
                .ok();
        }

        Ok(())
    }

    /// Parse a file containing URL-output pairs and download them
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to file containing URL-output pairs (one per line)
    ///
    /// # File Format
    ///
    /// Each line should contain either:
    /// - A single URL (filename will be derived from URL)
    /// - URL and output path separated by whitespace
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub async fn download_from_file(
        &self,
        file_path: PathBuf,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let pairs = parse_file(&file_path).await?;
        let downloads: Vec<(&str, PathBuf)> = pairs
            .iter()
            .map(|(url, output)| (url.as_str(), PathBuf::from(output)))
            .collect();

        self.download_multiple(downloads).await
    }
}

/// Initialize the library
///
/// This function should be called before using any library functions
/// to initialize internationalization and logging features.
pub fn init() {
    let _ = env_logger::try_init();
}

/// Re-export commonly used types and functions
pub use notifications::{notify_send, spawn_background_process};

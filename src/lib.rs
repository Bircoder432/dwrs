// src/lib.rs
//! A powerful and flexible file downloader library with parallel downloads,
//! resumable downloads, progress tracking, and desktop notifications.
//!
//! # Features
//!
//! - **Parallel downloads**: Split large files into chunks and download concurrently
//! - **Resumable downloads**: Continue interrupted downloads from where they left off
//! - **Progress tracking**: Visual progress bars with download statistics
//! - **Desktop notifications**: Get notified when downloads complete or fail
//! - **File-based batch downloads**: Download multiple files from a list
//! - **Configurable concurrency**: Control number of workers and parallel files
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use dwrs::{Downloader, DownloadConfig};
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     // Initialize logging
//!     dwrs::init();
//!
//!     // Create downloader with default config
//!     let downloader = Downloader::new_default();
//!
//!     // Download a single file
//!     downloader.download_file(
//!         "https://example.com/file.zip",
//!         PathBuf::from("file.zip")
//!     ).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Configuration
//!
//! Use [`DownloadConfig`] to customize behavior:
//!
//! ```
//! use dwrs::DownloadConfig;
//!
//! let config = DownloadConfig {
//!     workers: 8,              // Parallel chunks per file
//!     buffer_size: 512 * 1024, // 512KB buffer
//!     retries: 5,              // Retry failed downloads
//!     ..Default::default()
//! };
//! ```

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub mod cli;
pub mod config;
pub mod download;
pub mod file_parser;
#[cfg(feature = "notify")]
pub mod notifications;
pub mod progress;
pub mod utils;

use futures::stream::{FuturesUnordered, StreamExt};
use indicatif::MultiProgress;
use reqwest::{Client, ClientBuilder};
use tokio::sync::{Semaphore, mpsc};

pub use download::download_file;
pub use file_parser::parse_file;

/// Creates an optimized HTTP client with connection pooling and compression.
///
/// # Arguments
///
/// * `pool_size` - Maximum idle connections per host
///
/// # Features Enabled
///
/// - Connection pooling (up to `pool_size` idle connections per host)
/// - Gzip, Brotli, and Deflate compression
/// - TCP_NODELAY for reduced latency
/// - Automatic redirects (up to 10 hops)
/// - Custom user agent
///
/// # Timeouts
///
/// - Connection timeout: 30 seconds
/// - Request timeout: 5 minutes
pub fn create_optimized_client(pool_size: usize) -> Client {
    ClientBuilder::new()
        .pool_max_idle_per_host(pool_size)
        .timeout(Duration::from_secs(300))
        .connect_timeout(Duration::from_secs(30))
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .tcp_nodelay(true)
        .redirect(reqwest::redirect::Policy::limited(10))
        .user_agent(concat!("dwrs/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("Failed to build HTTP client")
}

/// Configuration for download operations.
///
/// Controls behavior of parallel downloads, retry logic, buffer sizes,
/// and UI customization.
///
/// # Examples
///
/// Default configuration:
/// ```
/// use dwrs::DownloadConfig;
///
/// let config = DownloadConfig::default();
/// ```
///
/// Custom workers and buffer size:
/// ```
/// use dwrs::DownloadConfig;
///
/// let config = DownloadConfig {
///     workers: 8,
///     buffer_size: 1024 * 1024, // 1MB
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct DownloadConfig {
    /// Number of parallel workers (chunks) per file download.
    ///
    /// Larger files are split into this many concurrent chunks.
    /// Minimum effective value is 1, maximum is calculated based on file size.
    ///
    /// Default: 4
    pub workers: usize,

    /// Whether to resume interrupted downloads.
    ///
    /// When true, existing partial files are detected and download
    /// continues from the last byte. Requires server support for
    /// HTTP Range requests.
    ///
    /// Default: false
    pub continue_download: bool,

    /// Enable desktop notifications on completion/failure.
    ///
    /// Requires the `notify` feature to be enabled.
    ///
    /// Default: false
    #[cfg(feature = "notify")]
    pub notify: bool,

    /// Progress bar template string.
    ///
    /// Uses indicatif template syntax. Available variables:
    /// - `{spinner}` - Animated spinner
    /// - `{elapsed_precise}` - Elapsed time
    /// - `{bar}` - Progress bar
    /// - `{pos}` / `{len}` - Current/total bytes
    /// - `{percent}` - Percentage complete
    /// - `{msg}` - Custom message
    ///
    /// Default: `"{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} ({percent}%) {msg}"`
    pub template: String,

    /// Message template for download start.
    ///
    /// Available variables: `{download}`, `{url}`, `{output}`
    ///
    /// Default: `"{download} {url} → {output}"`
    pub msg_template: String,

    /// Progress bar character set.
    ///
    /// Three characters: full, partial, empty
    /// Default: `"█▌░"`
    pub chars: String,

    /// Buffer size for file I/O in bytes.
    ///
    /// Larger buffers reduce system calls but use more memory.
    /// Recommended: 64KB to 1MB.
    ///
    /// Default: 262144 (256KB)
    pub buffer_size: usize,

    /// Maximum idle connections per host in the connection pool.
    ///
    /// Higher values improve performance for many downloads from
    /// the same host, but use more memory.
    ///
    /// Default: 100
    pub pool_size: usize,

    /// Number of retry attempts for failed downloads.
    ///
    /// Retries use exponential backoff: 2^attempt seconds delay.
    ///
    /// Default: 3
    pub retries: usize,

    /// Minimum file size in bytes to trigger parallel chunk downloading.
    ///
    /// Files smaller than this use single-threaded download.
    ///
    /// Default: 5242880 (5MB)
    pub min_parallel_size: u64,

    /// Maximum number of concurrent file downloads.
    ///
    /// When downloading multiple files, this limits how many
    /// download simultaneously. `None` enables auto-calculation
    /// based on worker count.
    ///
    /// Default: None (auto)
    pub max_concurrent_files: Option<usize>,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            workers: 4,
            template: "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} ({percent}%) {msg}".to_string(),
            msg_template: "{download} {url} → {output}".to_string(),
            chars: "█▌░".to_string(),
            continue_download: false,
            #[cfg(feature = "notify")]
            notify: false,
            buffer_size: 256 * 1024,
            pool_size: 100,
            retries: 3,
            min_parallel_size: 5 * 1024 * 1024,
            max_concurrent_files: None,
        }
    }
}

/// Main downloader struct managing HTTP client and configuration.
///
/// [`Downloader`] is the primary interface for downloading files.
/// It maintains an internal HTTP client with connection pooling
/// and provides methods for single and batch downloads.
///
/// # Thread Safety
///
/// [`Downloader`] is not `Send` due to internal progress bar handles.
/// Create a new instance per task or use [`Downloader::new`] with
/// cloned config for concurrent operations.
///
/// # Examples
///
/// Single file download:
/// ```rust,no_run
/// use dwrs::Downloader;
/// use std::path::PathBuf;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
/// let downloader = Downloader::new_default();
/// downloader.download_file(
///     "https://example.com/file.zip",
///     PathBuf::from("file.zip")
/// ).await?;
/// # Ok(())
/// # }
/// ```
///
/// Batch download with custom config:
/// ```rust,no_run
/// use dwrs::{Downloader, DownloadConfig};
/// use std::path::PathBuf;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
/// let config = DownloadConfig {
///     workers: 8,
///     max_concurrent_files: Some(4),
///     ..Default::default()
/// };
/// let downloader = Downloader::new(config);
///
/// let files: Vec<(&str, PathBuf)> = vec![
///     ("https://example.com/a.zip", PathBuf::from("a.zip")),
///     ("https://example.com/b.zip", PathBuf::from("b.zip")),
/// ];
///
/// downloader.download_multiple(files).await?;
/// # Ok(())
/// # }
/// ```
pub struct Downloader {
    config: DownloadConfig,
    client: Client,
}

impl Downloader {
    /// Creates a new [`Downloader`] with the specified configuration.
    ///
    /// Initializes an HTTP client with connection pooling based on
    /// [`DownloadConfig::pool_size`].
    ///
    /// # Panics
    ///
    /// Panics if the HTTP client fails to build (extremely rare).
    ///
    /// # Examples
    ///
    /// ```
    /// use dwrs::{Downloader, DownloadConfig};
    ///
    /// let config = DownloadConfig::default();
    /// let downloader = Downloader::new(config);
    /// ```
    pub fn new(config: DownloadConfig) -> Self {
        log::info!(
            "Creating Downloader: workers={}, buffer_size={}, pool_size={}",
            config.workers,
            config.buffer_size,
            config.pool_size
        );
        let client = create_optimized_client(config.pool_size);
        Self { config, client }
    }

    /// Creates a [`Downloader`] with default configuration.
    ///
    /// Convenience method equivalent to `Downloader::new(DownloadConfig::default())`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dwrs::Downloader;
    ///
    /// let downloader = Downloader::new_default();
    /// ```
    pub fn new_default() -> Self {
        Self::new(DownloadConfig::default())
    }

    /// Downloads a single file with automatic retry.
    ///
    /// Attempts download up to [`DownloadConfig::retries`] times with
    /// exponential backoff. Supports resume if enabled in config and
    /// server supports Range requests.
    ///
    /// # Arguments
    ///
    /// * `url` - HTTP(S) URL of the file to download
    /// * `output_path` - Local path where file should be saved
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error with the last failure reason.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use dwrs::Downloader;
    /// use std::path::PathBuf;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// let downloader = Downloader::new_default();
    /// downloader.download_file(
    ///     "https://example.com/file.zip",
    ///     PathBuf::from("downloads/file.zip")
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn download_file(
        &self,
        url: &str,
        output_path: PathBuf,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        log::info!(
            "Downloading single file: {} -> {}",
            url,
            output_path.display()
        );
        let mut last_error = None;

        for attempt in 0..self.config.retries {
            if attempt > 0 {
                let delay = 2u64.pow(attempt as u32);
                log::warn!(
                    "Retrying {} (attempt {}/{}), waiting {}s",
                    url,
                    attempt + 1,
                    self.config.retries,
                    delay
                );
                tokio::time::sleep(Duration::from_secs(delay)).await;
            }

            match self.try_download_single(url, &output_path).await {
                Ok(_) => {
                    log::info!("Download successful: {}", url);
                    return Ok(());
                }
                Err(e) => {
                    log::error!("Attempt {} failed for {}: {}", attempt + 1, url, e);
                    last_error = Some(e);

                    if attempt == 0 && output_path.exists() {
                        if let Ok(meta) = tokio::fs::metadata(&output_path).await {
                            if let Ok(head) = self.client.head(url).send().await {
                                if let Some(len) =
                                    head.headers().get(reqwest::header::CONTENT_LENGTH)
                                {
                                    if let Ok(total) = len.to_str().unwrap_or("0").parse::<u64>() {
                                        if meta.len() == total {
                                            log::info!("File already complete, skipping: {}", url);
                                            return Ok(());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| "Unknown error".into()))
    }

    /// Internal method for single download attempt.
    ///
    /// Creates progress bar and delegates to [`download::download_file`].
    /// Handles notification on completion if enabled.
    async fn try_download_single(
        &self,
        url: &str,
        output_path: &PathBuf,
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

        let result = download::download_file(
            &self.client,
            url,
            &output_path,
            &pb,
            self.config.continue_download,
            self.config.workers,
            self.config.buffer_size,
            self.config.min_parallel_size,
        )
        .await;

        #[cfg(feature = "notify")]
        if self.config.notify {
            use notify_rust::Notification;
            match &result {
                Ok(_) => {
                    Notification::new()
                        .summary("Download Complete")
                        .body(&format!("Finished: {}", output_path.display()))
                        .show()
                        .ok();
                }
                Err(e) => {
                    Notification::new()
                        .summary("Download Failed")
                        .body(&format!("{}: {}", output_path.display(), e))
                        .show()
                        .ok();
                }
            }
        }

        result
    }

    /// Downloads multiple files in parallel with concurrency limiting.
    ///
    /// Files are downloaded concurrently up to the limit specified by
    /// [`DownloadConfig::max_concurrent_files`] (or auto-calculated).
    /// Each file uses its own progress bar in a multi-progress display.
    ///
    /// # Arguments
    ///
    /// * `downloads` - Vector of (URL, output_path) pairs
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all downloads succeed, or an error listing
    /// all failed downloads.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use dwrs::Downloader;
    /// use std::path::PathBuf;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// let downloader = Downloader::new_default();
    ///
    /// let downloads: Vec<(&str, PathBuf)> = vec![
    ///     ("https://example.com/a.zip", PathBuf::from("a.zip")),
    ///     ("https://example.com/b.zip", PathBuf::from("b.zip")),
    ///     ("https://example.com/c.zip", PathBuf::from("c.zip")),
    /// ];
    ///
    /// downloader.download_multiple(downloads).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn download_multiple(
        &self,
        downloads: Vec<(&str, PathBuf)>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if downloads.is_empty() {
            log::warn!("No downloads to process");
            return Ok(());
        }

        log::info!("Starting batch download: {} files", downloads.len());
        let mp = Arc::new(MultiProgress::new());

        let max_concurrent = self.config.max_concurrent_files.unwrap_or_else(|| {
            let calculated = std::cmp::min(
                8,
                std::cmp::max(1, 16 / std::cmp::max(1, self.config.workers)),
            );
            log::debug!("Auto-calculated max_concurrent_files: {}", calculated);
            calculated
        });

        let semaphore = Arc::new(Semaphore::new(max_concurrent));
        let (tx, mut rx) = mpsc::unbounded_channel::<Result<(), String>>();

        let mut tasks = FuturesUnordered::new();
        let total = downloads.len();
        let mut errors = Vec::new();

        for (url, output_path) in downloads {
            let sem = semaphore.clone();
            let client = self.client.clone();
            let mp = mp.clone();
            let config = self.config.clone();
            let tx = tx.clone();
            let url_owned = url.to_string();

            let task = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();

                let pb = progress::create_progress_bar(
                    &mp,
                    &config.template,
                    &config.msg_template,
                    &config.chars,
                    &url_owned,
                    &output_path.to_string_lossy(),
                );

                let result = download::download_file(
                    &client,
                    &url_owned,
                    &output_path,
                    &pb,
                    config.continue_download,
                    config.workers,
                    config.buffer_size,
                    config.min_parallel_size,
                )
                .await;

                match result {
                    Ok(_) => {
                        pb.finish_with_message(format!("✓ {}", output_path.display()));
                        let _ = tx.send(Ok(()));
                    }
                    Err(e) => {
                        let error_msg = format!("✗ {}: {}", output_path.display(), e);
                        pb.finish_with_message(error_msg);
                        let _ = tx.send(Err(format!("{}: {}", url_owned, e)));
                    }
                }
            });

            tasks.push(task);
        }

        drop(tx);

        while let Some(result) = tasks.next().await {
            if let Err(e) = result {
                log::error!("Task panicked: {}", e);
                errors.push(format!("Task panicked: {}", e));
            }

            while let Ok(msg) = rx.try_recv() {
                if let Err(e) = msg {
                    log::error!("Download failed: {}", e);
                    errors.push(e);
                }
            }
        }

        while let Some(msg) = rx.recv().await {
            if let Err(e) = msg {
                log::error!("Download failed: {}", e);
                errors.push(e);
            }
        }

        if !errors.is_empty() {
            log::error!(
                "Batch download failed: {}/{} files failed",
                errors.len(),
                total
            );
            return Err(format!(
                "{}/{} downloads failed:\n{}",
                errors.len(),
                total,
                errors.join("\n")
            )
            .into());
        }

        log::info!(
            "Batch download complete: {}/{} files successful",
            total,
            total
        );
        Ok(())
    }

    /// Downloads files listed in a text file.
    ///
    /// File format: one URL per line, optionally followed by output filename.
    /// Lines starting with `#` are treated as comments.
    ///
    /// # File Format Example
    ///
    /// ```text
    /// # Comments start with #
    /// https://example.com/file1.zip output1.zip
    /// https://example.com/file2.zip
    /// https://example.com/file3.zip output3.zip
    /// ```
    ///
    /// When output name is omitted, it's derived from the URL path.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to text file containing URL list
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if file cannot be read
    /// or contains no valid URLs.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use dwrs::Downloader;
    /// use std::path::PathBuf;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// let downloader = Downloader::new_default();
    /// downloader.download_from_file(PathBuf::from("downloads.txt")).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn download_from_file(
        &self,
        file_path: PathBuf,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        log::info!("Loading URLs from file: {}", file_path.display());
        let pairs = parse_file(&file_path).await?;
        log::info!("Loaded {} URLs from file", pairs.len());

        let downloads: Vec<(&str, PathBuf)> = pairs
            .iter()
            .map(|(url, output)| (url.as_str(), PathBuf::from(output)))
            .collect();

        self.download_multiple(downloads).await
    }
}

/// Initializes the library logging system.
///
/// Attempts to initialize `env_logger`. Safe to call multiple times;
/// subsequent calls are ignored.
///
/// # Examples
///
/// ```
/// // Call at start of main()
/// dwrs::init();
/// ```
pub fn init() {
    let _ = env_logger::try_init();
    log::info!("dwrs initialized");
}

/// Notification utilities for desktop alerts.
///
/// Requires the `notify` feature to be enabled at compile time.
#[cfg(feature = "notify")]
pub use notifications::{notify_send, spawn_background_process};

// System and library imports
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::Arc;

use once_cell::sync::Lazy;               // For lazy static initialization
use rust_i18n::t;                         // Localization macro
use locale_config::Locale;               // System locale detection

// CLI, formatting, concurrency, logging, progress and HTTP libraries
use clap::Parser;
use colored::*;                           // Terminal color styling
use futures::stream::{FuturesUnordered, StreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle}; // Progress bar utilities
use log::{error, info};                  // Logging macros
use reqwest::Client;                     // HTTP client
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Semaphore;             // Concurrency limit
use tokio::task;


rust_i18n::i18n!("i18n"); // Loads localization YAML files from the `i18n` directory

// Lazy static initializer to set the application's locale before CLI parsing
static INIT_LOCALE: Lazy<()> = Lazy::new(|| {
    let system_locale = Locale::user_default().to_string();
    let short_locale = system_locale.split('_').next().unwrap_or("en");
    rust_i18n::set_locale(short_locale);
});

/// CLI argument definition using clap
#[derive(Parser)]
#[command(name = "dwrs",author, version, about = format!("{}",t!("about")))]
#[command(group(clap::ArgGroup::new("input").required(true).args(&["url","file"])))]
struct Args {
    /// Continue download
    #[arg(short, long, default_value_t = false)]
    continue_: bool,
    /// Direct list of URLs to download
    #[arg(required = false)]
    url: Vec<String>,

    /// Output filenames (must match count of URLs or be empty)
    #[arg(short, long)]
    output: Vec<String>,

    /// Max number of parallel downloads
    #[arg(short, long, default_value = "1")]
    jobs: usize,

    /// Optional input file in format: `url output`
    #[arg(short, long)]
    file: Option<PathBuf>,
}

/// Parses a file with either `url output` or `url` lines
async fn parse_file(path: &PathBuf) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    let file = File::open(path).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let mut pairs = Vec::new();

    while let Some(line) = lines.next_line().await? {
        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.len() == 2 {
            // Explicit output filename
            pairs.push((parts[0].to_string(), parts[1].to_string()));
        } else if parts.len() == 1 {
            // Auto-generate filename from URL
            let filename = parts[0].split('/').last().unwrap_or("file.bin").to_string();
            pairs.push((parts[0].to_string(), filename));
        } else {
            // Malformed line
            eprintln!("{}: {}", t!("wrong-format-string").red().bold(), line);
        }
    }
    Ok(pairs)
}

#[tokio::main]
async fn main() {
    Lazy::force(&INIT_LOCALE); // Force locale setup before anything else
    env_logger::init(); // Enable logging
    info!("logger init");


    let args = Args::parse(); // Parse command-line arguments

    let mut url_output_pairs = Vec::new();

    // Load from file or from direct CLI arguments
    if let Some(file_path) = args.file {
        url_output_pairs = parse_file(&file_path).await.unwrap_or_else(|e| {
            eprintln!("{}: {}", t!("error-in-reading-file").red().bold(), e);
            panic!();
        });
    } else {
        for (i, url) in args.url.iter().enumerate() {
            let output = if let Some(path) = args.output.get(i) {
                path.clone()
            } else {
                url.split('/').last().unwrap_or("file.bin").to_string()
            };
            url_output_pairs.push((url.clone(), output));
        }

        // Validate that number of output paths matches number of URLs
        if !args.output.is_empty() && args.output.len() != args.url.len() {
            println!("{}", args.output.len());
            error!("Error count of output files dont equal of urls");
            eprintln!("{}", t!("error-count").red().bold());
            panic!()
        }
    }

    // Shared HTTP client and progress manager
    let client = Client::new();
    let mp = Arc::new(MultiProgress::new());
    let semaphore = Arc::new(Semaphore::new(args.jobs));

    let mut tasks = FuturesUnordered::new();

    // Spawn async download tasks
    for (url, output) in url_output_pairs.into_iter() {
        let outstr = output.clone();
        let output = PathBuf::from(output);

        let client = client.clone();
        let mp = mp.clone();
        let sem = semaphore.clone();
        let url = url.clone();
        let resume = args.continue_;
        tasks.push(task::spawn(async move {
            let _permit = sem.acquire().await.unwrap(); // Wait for slot
            let pb = mp.add(ProgressBar::new_spinner()); // Spinner-based progress bar
            pb.set_style(
                ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} ({percent}%) {msg}").unwrap()
            );

            pb.set_message(
                format!(
                    "{} {} → {}",
                    t!("download").blue(),
                        url.yellow().bold(),
                        outstr.green().bold()
                )
            );

            match download_file(&client, &url , &output, &pb, resume).await {
                Ok(_) => pb.finish_with_message(
                    format!("{}: {}", t!("download-finish").green().bold(), outstr.green())
                ),
                Err(e) => pb.finish_with_message(
                    format!(
                        "{}: {}: {}",
                        t!("download-error").red().bold(),
                            outstr,
                            e
                    )
                ),
            }
        }));
    }

    // Wait for all tasks to complete
    while let Some(_) = tasks.next().await {}
}

/// Download a single file with a streaming response and write to disk
async fn download_file(
    client: &Client,
    url: &str,
    output: &PathBuf,
    pb: &ProgressBar,
    resume: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut start: u64 = 0;
    if resume && output.exists() {
        start = tokio::fs::metadata(&output).await?.len();
    }

    let mut request = client.get(url);
    if start > 0 {
        request = request.header("Range", format!("bytes={}-",start));
    }
    let response = request.send().await?;
    if response.status().as_u16() != 200 as u16 {
        error!("Error status code of response: {}", response.status().as_u16());
        panic!("Status code {}",response.status().as_str().red().bold())
    }
    let file_size = response.content_length().unwrap_or(0);

    pb.set_length(start + file_size);
    pb.set_position(start);
    let mut file = if start > 0 {
        tokio::fs::OpenOptions::new()
            .append(true)
            .open(output)
            .await?
    } else {
        tokio::fs::File::create(output).await?
    };
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        pb.inc(chunk.len() as u64); // Update progress
    }


    Ok(())
}

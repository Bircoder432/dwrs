use clap::Parser;
use colored::*;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use locale_config::Locale;
use once_cell::sync::Lazy;
use reqwest::blocking::Client;
use rust_i18n::t;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write, Read};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

rust_i18n::i18n!("i18n");

// Lazy static initializer to set the application locale at startup
static INIT_LOCALE: Lazy<()> = Lazy::new(|| {
    let system_locale = Locale::user_default().to_string();
    let short_locale = system_locale.split('_').next().unwrap_or("en");
    rust_i18n::set_locale(short_locale);
});

/// Definition of CLI arguments
#[derive(Parser)]
#[command(name = "dwrs", author, version, about = format!("{}", t!("about")))]
#[command(group(clap::ArgGroup::new("input").required(true).args(&["url", "file"])))]
struct Args {
     /// Direct list of URLs to download
    #[arg(required = false)]
    url: Vec<String>,

    /// Output file names (optional)
    #[arg(short, long)]
    output: Vec<String>,

    /// Number of parallel download jobs
    #[arg(short, long, default_value = "1")]
    jobs: usize,

    /// Input file containing lines of: `url output`
    #[arg(short, long)]
    file: Option<PathBuf>,
}

/// Parse a file with either `url output` or just `url` per line
fn parse_file(path: &PathBuf) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut pairs = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.len() == 2 {
            // Line with explicit output filename
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


fn download_file(
    client: &Client,
    url: &str,
    output: &PathBuf,
    pb: &ProgressBar,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut response = client.get(url).send()?;
    let total_size = response.content_length().unwrap_or(0);

    pb.set_length(total_size);

    let mut file = File::create(output)?;
    let mut downloaded = 0u64;
    let mut buffer = [0u8; 8192];

    while let Ok(n) = response.read(&mut buffer) {
        if n == 0 {
            break;
        }
        file.write_all(&buffer[..n])?;
        downloaded += n as u64;
        pb.set_position(downloaded);
    }

    Ok(())
}

fn main() {
    Lazy::force(&INIT_LOCALE);
    env_logger::init();

    let args = Args::parse();
    let client = Client::new();
    let mp = Arc::new(MultiProgress::new());
    let semaphore = Arc::new(Mutex::new(0usize));

    let mut url_output_pairs = Vec::new();

    if let Some(file_path) = args.file {
        url_output_pairs = parse_file(&file_path).unwrap_or_else(|e| {
            eprintln!("{}: {}", t!("error-in-reading-file").red().bold(), e);
            std::process::exit(1);
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

        if !args.output.is_empty() && args.output.len() != args.url.len() {
            eprintln!("{}", t!("error-count").red().bold());
            std::process::exit(1);
        }
    }

    let mut handles = vec![];

    for (url, output) in url_output_pairs.into_iter() {
        let client = client.clone();
        let mp = mp.clone();
        let sem = semaphore.clone();
        let output = PathBuf::from(output);
        let outstr = output.display().to_string();

        // wait for free slot
        loop {
            let mut count = sem.lock().unwrap();
            if *count < args.jobs {
                *count += 1;
                break;
            }
            drop(count);
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        let handle = thread::spawn(move || {
            let pb = mp.add(ProgressBar::new_spinner());
            pb.set_style(
                ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} ({percent}%) {msg}")
                .unwrap(),
            );
            pb.set_message(format!(
                "{} {} → {}",
                t!("download").blue(),
                                   url.yellow().bold(),
                                   outstr.green().bold()
            ));

            match download_file(&client, &url, &output, &pb) {
                Ok(_) => {
                    pb.finish_with_message(format!(
                        "{}: {}",
                        t!("download-finish").green().bold(),
                                                   outstr.green()
                    ));
                }
                Err(e) => {
                    pb.finish_with_message(format!(
                        "{}: {}: {}",
                        t!("download-error").red().bold(),
                                                   outstr,
                                                   e
                    ));
                }
            }

            let mut count = sem.lock().unwrap();
            *count -= 1;
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

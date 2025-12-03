#[macro_use]
extern crate rust_i18n;

i18n!("locale", fallback = "en");

use std::sync::Arc;

mod cli;
mod download;
mod file_parser;
mod notifications;
mod progress;

use clap::Parser;
use cli::Args;
use colored::Colorize;
use dwrs::*;
use futures::stream::{FuturesUnordered, StreamExt};
use indicatif::MultiProgress;
use log::{error, info};
use reqwest::Client;
use rust_i18n::t;
use tokio::sync::Semaphore;
use tokio::task;

#[tokio::main]
async fn main() {
    init();
    info!("Logger initialized");

    let args = Args::parse();

    if args.background {
        spawn_background_process().unwrap();
        return;
    }

    let mut url_output_pairs = Vec::new();

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

        if !args.output.is_empty() && args.output.len() != args.url.len() {
            error!("Error count of output files dont equal of urls");
            eprintln!("{}", t!("error-count").red().bold());
            panic!()
        }
    }

    let client = Client::new();
    let mp = Arc::new(MultiProgress::new());
    let semaphore = Arc::new(Semaphore::new(args.jobs));
    let mut tasks = FuturesUnordered::new();

    for (url, output) in url_output_pairs.into_iter() {
        let outstr = output.clone();
        let output = std::path::PathBuf::from(output);

        let client = client.clone();
        let mp = mp.clone();
        let sem = semaphore.clone();
        let url = url.clone();
        let resume = args.continue_;
        let notify = args.notify;
        let jobs = args.jobs;

        tasks.push(task::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let pb = progress::create_progress_bar(&mp, &url, &outstr);

            match download_file(&client, &url, &output, &pb, resume, jobs).await {
                Ok(_) => {
                    pb.finish_and_clear();
                    pb.finish_with_message(format!(
                        "{}: {}",
                        t!("download-finish").green().bold(),
                        outstr.green()
                    ));
                    if notify {
                        notify_send(t!("download-finish").to_string());
                    }
                }
                Err(e) => {
                    pb.finish_with_message(format!(
                        "{}: {}: {}",
                        t!("download-error").red().bold(),
                        outstr,
                        e
                    ));
                    if notify {
                        notify_send(format!("{}: {}: {}", t!("download-error"), outstr, e));
                    }
                }
            }
        }));
    }

    while let Some(_) = tasks.next().await {}
}

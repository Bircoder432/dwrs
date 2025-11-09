use std::sync::Arc;

mod cli;
mod download;
mod file_parser;
mod localization;
mod notifications;
mod progress;

use clap::Parser;
use cli::Args;
use colored::Colorize;
use download::download_file;
use file_parser::parse_file;
use futures::stream::{FuturesUnordered, StreamExt};
use indicatif::MultiProgress;
use localization::init_locale;
use log::{error, info};
use notifications::{notify_send, spawn_background_process};
use progress::create_progress_bar;
use reqwest::Client;
use rust_i18n::t;
use tokio::sync::Semaphore;
use tokio::task;

rust_i18n::i18n!("i18n");

#[tokio::main]
async fn main() {
    init_locale();
    env_logger::init();
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

        tasks.push(task::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let pb = create_progress_bar(&mp, &url, &outstr);

            match download_file(&client, &url, &output, &pb, resume).await {
                Ok(_) => {
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

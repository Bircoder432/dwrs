use colored::Colorize;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub fn create_progress_bar(mp: &MultiProgress, url: &str, output: &str) -> ProgressBar {
    let pb = mp.add(ProgressBar::new_spinner());
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} ({percent}%) {msg}").unwrap()
    );

    pb.set_message(format!(
        "Downloading {} → {}",
        url.yellow().bold(),
        output.green().bold()
    ));

    pb
}

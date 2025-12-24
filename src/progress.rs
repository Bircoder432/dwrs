use colored::Colorize;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub fn create_progress_bar(
    mp: &MultiProgress,
    template: &str,
    chars: &str,
    url: &str,
    output: &str,
) -> ProgressBar {
    let pb = mp.add(ProgressBar::new_spinner());
    pb.set_style(
        ProgressStyle::with_template(template)
            .unwrap()
            .progress_chars(chars),
    );

    pb.set_message(format!(
        "{} {} → {}",
        t!("download").green().bold(),
        url.yellow().bold(),
        output.green().bold()
    ));

    pb
}

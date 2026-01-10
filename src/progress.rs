use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use std::{borrow::Cow, collections::HashMap};

use crate::utils::{parse_template, render};

pub fn create_progress_bar(
    mp: &MultiProgress,
    template: &str,
    msg_template: &str,
    chars: &str,
    url: &str,
    output: &str,
) -> ProgressBar {
    let pb = mp.add(ProgressBar::new_spinner());

    pb.set_style(
        ProgressStyle::with_template(template)
            .unwrap_or_else(|_| ProgressStyle::default_bar())
            .progress_chars(chars),
    );

    let tokens = parse_template(msg_template);

    let vars: HashMap<&str, Cow<'_, str>> = HashMap::from([
        ("download", Cow::Owned(t!("download").to_string())),
        ("url", Cow::Borrowed(url)),
        ("output", Cow::Borrowed(output)),
    ]);

    let message = render(&tokens, &vars);
    pb.set_message(message);

    pb
}

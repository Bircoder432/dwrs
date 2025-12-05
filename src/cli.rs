use clap::Parser;
use lazy_static::lazy_static;
use std::path::PathBuf;

lazy_static! {
    static ref ABOUT_TEXT: String = rust_i18n::t!("about").to_string();
}

#[derive(Parser)]
#[command(name = "dwrs", author, version, about = ABOUT_TEXT.as_str())]
#[command(group(clap::ArgGroup::new("input").required(true).args(&["url","file"])))]
pub struct Args {
    #[arg(short, long)]
    pub notify: bool,

    #[arg(long)]
    pub background: bool,

    #[arg(short, long, default_value_t = false)]
    pub continue_: bool,

    #[arg(required = false)]
    pub url: Vec<String>,

    #[arg(short, long)]
    pub output: Vec<String>,

    #[arg(short, long, default_value = "1")]
    pub workers: usize,

    #[arg(short, long)]
    pub file: Option<PathBuf>,
}

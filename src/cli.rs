use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "dwrs", author, version, about = format!("{}", rust_i18n::t!("about")))]
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
    pub jobs: usize,

    #[arg(short, long)]
    pub file: Option<PathBuf>,
}

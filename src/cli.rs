use clap::Parser;
use lazy_static::lazy_static;
use std::path::PathBuf;

lazy_static! {
    static ref ABOUT_TEXT: String =
        "A utility for parallel downloading of files from the internet with a progress bar"
            .to_string();
}

#[derive(Parser)]
#[command(name = "dwrs", author, version, about = ABOUT_TEXT.as_str())]
#[command(group(clap::ArgGroup::new("input").required(true).args(&["url","file"])))]
pub struct Args {
    #[cfg(feature = "notify")]
    #[arg(short, long)]
    pub notify: bool,
    // enable in background mode
    #[arg(long)]
    pub background: bool,
    // continue downloading from last position
    #[arg(short, long, default_value_t = false)]
    pub continue_: bool,
    // url of file to download
    #[arg(required = false)]
    pub url: Vec<String>,
    // output file name
    #[arg(short, long)]
    pub output: Vec<String>,
    // count of workers
    #[arg(short, long, default_value = "4")]
    pub workers: usize,
    // file for parsing
    #[arg(short, long)]
    pub file: Option<PathBuf>,
    // config file
    #[arg(long)]
    pub config: Option<String>,

    /// Buffer size in KB (default: 256)
    #[arg(long, value_name = "KB")]
    pub buffer_size: Option<usize>,

    /// Connection pool size per host
    #[arg(long, default_value = "100")]
    pub pool_size: usize,

    /// Retry failed downloads N times
    #[arg(short = 'r', long, default_value = "3")]
    pub retries: usize,

    /// Concurrent file limit (auto if not set)
    #[arg(long)]
    pub max_files: Option<usize>,

    /// Minimum file size in MB to use parallel chunk downloading
    #[arg(long, default_value = "5")]
    pub min_parallel_size: u64,
}

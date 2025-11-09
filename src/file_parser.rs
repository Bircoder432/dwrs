use colored::Colorize;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

pub async fn parse_file(
    path: &PathBuf,
) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send + Sync>> {
    let file = File::open(path).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut pairs = Vec::new();

    while let Some(line) = lines.next_line().await? {
        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.len() == 2 {
            pairs.push((parts[0].to_string(), parts[1].to_string()));
        } else if parts.len() == 1 {
            let filename = parts[0].split('/').last().unwrap_or("file.bin").to_string();
            pairs.push((parts[0].to_string(), filename));
        } else {
            eprintln!("{}: {}", "Wrong format string".red().bold(), line);
        }
    }
    Ok(pairs)
}

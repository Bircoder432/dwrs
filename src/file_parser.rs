use colored::Colorize;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

const FILE_BUFFER_SIZE: usize = 1024 * 1024;

pub async fn parse_file(
    path: &PathBuf,
) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send + Sync>> {
    let file = File::open(path)
        .await
        .map_err(|e| format!("Cannot open file {}: {}", path.display(), e))?;

    let reader = BufReader::with_capacity(FILE_BUFFER_SIZE, file);
    let mut lines = reader.lines();
    let mut pairs = Vec::with_capacity(1024);

    let mut line_num = 0;
    while let Some(line) = lines.next_line().await? {
        line_num += 1;

        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();

        match parts.len() {
            0 => continue,
            1 => {
                let url = parts[0];
                let filename = url
                    .rsplit('/')
                    .next()
                    .filter(|s| !s.is_empty())
                    .unwrap_or("file.bin");

                if !url.starts_with("http://") && !url.starts_with("https://") {
                    eprintln!(
                        "{}: line {} - invalid URL: {}",
                        "Warning".yellow(),
                        line_num,
                        url
                    );
                    continue;
                }

                pairs.push((url.to_string(), filename.to_string()));
            }
            _ => {
                let url = parts[0];
                let filename = parts[1];

                if !url.starts_with("http://") && !url.starts_with("https://") {
                    eprintln!(
                        "{}: line {} - invalid URL: {}",
                        "Warning".yellow(),
                        line_num,
                        url
                    );
                    continue;
                }

                pairs.push((url.to_string(), filename.to_string()));
            }
        }
    }

    pairs.shrink_to_fit();

    if pairs.is_empty() {
        return Err("No valid URLs found in file".into());
    }

    Ok(pairs)
}

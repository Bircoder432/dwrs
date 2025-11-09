use futures::StreamExt;
use indicatif::ProgressBar;
use log::error;
use reqwest::Client;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

pub async fn download_file(
    client: &Client,
    url: &str,
    output: &PathBuf,
    pb: &ProgressBar,
    resume: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut start: u64 = 0;
    if resume && output.exists() {
        start = tokio::fs::metadata(&output).await?.len();
    }

    let mut request = client.get(url);
    if start > 0 {
        request = request.header("Range", format!("bytes={}-", start));
    }

    let response = request.send().await?;
    if response.status().as_u16() != 200 {
        error!(
            "Error status code of response: {}",
            response.status().as_u16()
        );
        return Err(format!("Error status code: {}", response.status().as_str()).into());
    }

    let file_size = response.content_length().unwrap_or(0);
    pb.set_length(start + file_size);
    pb.set_position(start);

    let mut file = if start > 0 {
        tokio::fs::OpenOptions::new()
            .append(true)
            .open(output)
            .await?
    } else {
        tokio::fs::File::create(output).await?
    };

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        pb.inc(chunk.len() as u64);
    }

    Ok(())
}

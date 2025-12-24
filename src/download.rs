use futures::StreamExt;
use indicatif::ProgressBar;
use log::error;
use reqwest::Client;
use std::path::{Path, PathBuf};
use tokio::{fs, io::AsyncWriteExt, task};

pub async fn download_file(
    client: &Client,
    url: &str,
    output: &PathBuf,
    pb: &ProgressBar,
    resume: bool,
    workers: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let head_resp = client.head(url).send().await?;
    let total_size = head_resp
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok()?.parse::<u64>().ok())
        .unwrap_or(0);

    pb.set_length(total_size);

    if workers <= 1 || total_size == 0 {
        let end = if total_size > 0 { total_size - 1 } else { 0 };
        return download_range(client, url, output, pb, resume, 0, end).await;
    }

    let chunk_size = (total_size + workers as u64 - 1) / workers as u64;
    let mut handles = vec![];

    for i in 0..workers {
        let start = i as u64 * chunk_size;
        let end = std::cmp::min(start + chunk_size - 1, total_size - 1);
        let client = client.clone();
        let url = url.to_string();
        let tmp_path = output.with_extension(format!("part{}", i));
        let pb = pb.clone();
        let resume = resume;

        handles.push(task::spawn(async move {
            download_range(&client, &url, &tmp_path, &pb, resume, start, end).await?;
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(tmp_path)
        }));
    }

    let mut parts = vec![];
    for handle in handles {
        let tmp = handle.await??;
        parts.push(tmp);
    }

    let mut final_file = fs::File::create(output).await?;
    for part in &parts {
        let mut f = fs::File::open(part).await?;
        tokio::io::copy(&mut f, &mut final_file).await?;
        fs::remove_file(part).await.ok();
    }

    Ok(())
}

async fn download_range(
    client: &Client,
    url: &str,
    output: &Path,
    pb: &ProgressBar,
    resume: bool,
    start: u64,
    end: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut offset = start;

    if resume && output.exists() {
        offset += fs::metadata(output).await?.len();
    }

    if offset > end {
        pb.inc(end - start + 1);
        return Ok(());
    }

    let head_resp = client.head(url).send().await?;
    let accept_ranges = head_resp
        .headers()
        .get(reqwest::header::ACCEPT_RANGES)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let mut request = client.get(url);
    if accept_ranges == "bytes" {
        request = request.header("Range", format!("bytes={}-{}", offset, end));
    }

    let resp = request.send().await?;
    if !resp.status().is_success() && resp.status().as_u16() != 206 {
        error!("Error status code: {}", resp.status());
        return Err(format!("HTTP error: {}", resp.status()).into());
    }

    let mut file = if resume && offset > start {
        fs::OpenOptions::new().append(true).open(output).await?
    } else {
        fs::File::create(output).await?
    };

    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        pb.inc(chunk.len() as u64);
    }

    Ok(())
}

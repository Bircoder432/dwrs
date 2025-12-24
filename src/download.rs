use futures::StreamExt;
use indicatif::ProgressBar;
use reqwest::Client;
use std::path::{Path, PathBuf};
use tokio::{fs, io::AsyncWriteExt};

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

    let accept_ranges = head_resp
        .headers()
        .get(reqwest::header::ACCEPT_RANGES)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    pb.set_length(total_size);

    let use_range = accept_ranges == "bytes" && total_size > 0;
    if !use_range || workers <= 1 {
        return download_range(
            client,
            url,
            output,
            pb,
            resume,
            0,
            total_size.saturating_sub(1),
        )
        .await;
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

        handles.push(tokio::task::spawn(async move {
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
    let use_range = end > start;

    let mut offset = start;
    if resume && output.exists() {
        offset += fs::metadata(output).await?.len();
    }

    let mut request = client.get(url);
    if use_range {
        request = request.header("Range", format!("bytes={}-{}", offset, end));
    }

    let resp = request.send().await?;
    if !resp.status().is_success() && resp.status().as_u16() != 206 {
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

#[tokio::test]
async fn test_download_range_no_range() {
    use httpmock::MockServer;
    use indicatif::ProgressBar;
    use reqwest::Client;
    use std::path::PathBuf;
    let server = MockServer::start();
    let body = b"hello world";
    let m = server.mock(|when, then| {
        when.method("GET").path("/file.txt");
        then.status(200).header("Content-Length", "11").body(body);
    });

    let client = Client::new();
    let output = PathBuf::from("test_file.txt");
    let pb = ProgressBar::new(11);

    download_range(
        &client,
        &format!("{}/file.txt", server.url("")),
        &output,
        &pb,
        false,
        0,
        10,
    )
    .await
    .unwrap();

    let content = tokio::fs::read(&output).await.unwrap();
    assert_eq!(content, body);
    m.assert();
    tokio::fs::remove_file(output).await.ok();
}

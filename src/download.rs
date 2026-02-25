use futures::StreamExt;
use indicatif::ProgressBar;
use reqwest::Client;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::{fs, io::AsyncWriteExt};

const DEFAULT_BUFFER_SIZE: usize = 256 * 1024;
const STREAM_CHUNK_SIZE: usize = 64 * 1024;
const MIN_CHUNK_SIZE: u64 = 2 * 1024 * 1024;

pub async fn download_file(
    client: &Client,
    url: &str,
    output: &PathBuf,
    pb: &ProgressBar,
    resume: bool,
    workers: usize,
    buffer_size: usize,
    min_parallel_size: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    log::debug!("Starting download: {} -> {}", url, output.display());

    let head_resp = match client.head(url).send().await {
        Ok(resp) => {
            log::debug!(
                "HEAD request successful for {}: status {}",
                url,
                resp.status()
            );
            resp
        }
        Err(e) => {
            log::error!("HEAD request failed for {}: {}", url, e);
            return Err(format!("Failed to connect: {}", e).into());
        }
    };

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

    log::info!(
        "File: {}, Size: {} bytes, Accept-Ranges: {}",
        url,
        total_size,
        accept_ranges
    );

    pb.set_length(total_size);

    let use_parallel = accept_ranges == "bytes" && total_size > min_parallel_size && workers > 1;

    if !use_parallel {
        log::info!(
            "Using sequential download for {} (workers={}, size={}, threshold={})",
            url,
            workers,
            total_size,
            min_parallel_size
        );
        return download_optimized(client, url, output, pb, resume, total_size, buffer_size).await;
    }

    log::info!(
        "Using parallel download for {} with {} workers",
        url,
        workers
    );
    download_parallel(
        client,
        url,
        output,
        pb,
        resume,
        total_size,
        workers,
        buffer_size,
    )
    .await
}

async fn download_optimized(
    client: &Client,
    url: &str,
    output: &Path,
    pb: &ProgressBar,
    resume: bool,
    total_size: u64,
    buffer_size: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut start_byte = 0u64;

    if resume && output.exists() {
        match fs::metadata(output).await {
            Ok(meta) => {
                let existing = meta.len();
                log::debug!("Existing file size: {} bytes", existing);
                if existing < total_size {
                    start_byte = existing;
                    pb.set_position(start_byte);
                    log::info!("Resuming download from byte {}", start_byte);
                } else if existing == total_size {
                    log::info!("File already complete: {}", output.display());
                    pb.finish_with_message("Already complete");
                    return Ok(());
                } else {
                    log::warn!(
                        "Existing file larger than expected, removing: {}",
                        output.display()
                    );
                    fs::remove_file(output).await.ok();
                }
            }
            Err(e) => {
                log::warn!("Failed to read metadata for {}: {}", output.display(), e);
                fs::remove_file(output).await.ok();
            }
        }
    }

    let mut request = client.get(url);
    if start_byte > 0 {
        request = request.header("Range", format!("bytes={}-", start_byte));
        log::debug!("Adding Range header: bytes={}-", start_byte);
    }

    let resp = request.send().await?.error_for_status()?;
    log::debug!("GET request successful, status: {}", resp.status());

    let file = if resume && start_byte > 0 {
        fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(output)
            .await?
    } else {
        fs::File::create(output).await?
    };

    let mut writer = tokio::io::BufWriter::with_capacity(buffer_size, file);
    let mut stream = resp.bytes_stream();
    let mut downloaded = start_byte;
    let mut last_log = downloaded;
    let log_interval = 10 * 1024 * 1024;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let len = chunk.len() as u64;
        writer.write_all(&chunk).await?;
        downloaded += len;
        pb.set_position(downloaded);

        if downloaded - last_log >= log_interval {
            log::info!(
                "Downloaded {} MB / {} MB ({:.1}%)",
                downloaded / 1024 / 1024,
                total_size / 1024 / 1024,
                (downloaded as f64 / total_size as f64) * 100.0
            );
            last_log = downloaded;
        }
    }

    writer.flush().await?;
    log::info!(
        "Download complete: {} ({} bytes)",
        output.display(),
        downloaded
    );
    pb.finish();
    Ok(())
}

async fn download_parallel(
    client: &Client,
    url: &str,
    output: &PathBuf,
    pb: &ProgressBar,
    resume: bool,
    total_size: u64,
    workers: usize,
    buffer_size: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let optimal_workers = std::cmp::min(
        workers,
        std::cmp::max(1, (total_size / MIN_CHUNK_SIZE) as usize),
    );

    let chunk_size = total_size.div_ceil(optimal_workers as u64);
    log::info!(
        "Parallel download: {} chunks, {} bytes each",
        optimal_workers,
        chunk_size
    );

    let pb_shared = Arc::new(pb.clone());

    let mut handles = Vec::with_capacity(optimal_workers);
    let progress_shared = Arc::new(AtomicU64::new(pb.position()));

    for i in 0..optimal_workers {
        let start = i as u64 * chunk_size;
        let end = std::cmp::min(start + chunk_size - 1, total_size - 1);

        if start > end {
            continue;
        }

        let client = client.clone();
        let url = url.to_string();
        let tmp_path = output.with_extension(format!("part{}", i));
        let pb_clone = pb_shared.clone();
        let progress = progress_shared.clone();

        log::debug!("Spawning chunk {}: bytes {}-{}", i, start, end);

        handles.push(tokio::spawn(async move {
            download_chunk(
                &client,
                &url,
                &tmp_path,
                start,
                end,
                resume,
                pb_clone,
                progress,
                buffer_size,
            )
            .await
        }));
    }

    let mut parts = Vec::with_capacity(handles.len());
    for (i, handle) in handles.into_iter().enumerate() {
        match handle.await {
            Ok(Ok(path)) => {
                log::debug!("Chunk {} completed: {}", i, path.display());
                parts.push((i, path));
            }
            Ok(Err(e)) => {
                log::error!("Chunk {} failed: {}", i, e);
                return Err(format!("Chunk {} failed: {}", i, e).into());
            }
            Err(e) => {
                log::error!("Chunk {} panicked: {}", i, e);
                return Err(format!("Chunk {} panicked: {}", i, e).into());
            }
        }
    }

    parts.sort_by_key(|(i, _)| *i);
    let sorted_parts: Vec<_> = parts.into_iter().map(|(_, p)| p).collect();

    log::info!(
        "Merging {} chunks into {}",
        sorted_parts.len(),
        output.display()
    );
    merge_parts(output, &sorted_parts, total_size).await?;

    pb.finish();
    Ok(())
}

async fn download_chunk(
    client: &Client,
    url: &str,
    tmp_path: &Path,
    start: u64,
    end: u64,
    resume: bool,
    pb: Arc<ProgressBar>,
    progress: Arc<AtomicU64>,
    buffer_size: usize,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let chunk_size = end.saturating_sub(start) + 1;
    let mut current_start = start;

    if resume && tmp_path.exists() {
        match fs::metadata(tmp_path).await {
            Ok(meta) => {
                let existing = meta.len();
                if existing > 0 && existing < chunk_size {
                    current_start = start + existing;
                    log::debug!("Resuming chunk from byte {}", current_start);
                } else if existing >= chunk_size {
                    log::debug!("Chunk already complete: {}", tmp_path.display());
                    return Ok(tmp_path.to_path_buf());
                } else {
                    fs::remove_file(tmp_path).await.ok();
                }
            }
            Err(_) => {
                fs::remove_file(tmp_path).await.ok();
            }
        }
    }

    let request = client
        .get(url)
        .header("Range", format!("bytes={}-{}", current_start, end))
        .send()
        .await?
        .error_for_status()?;

    let file = if resume && current_start > start && tmp_path.exists() {
        fs::OpenOptions::new().append(true).open(tmp_path).await?
    } else {
        fs::File::create(tmp_path).await?
    };

    let mut writer = tokio::io::BufWriter::with_capacity(
        std::cmp::min(buffer_size / 4, STREAM_CHUNK_SIZE * 4),
        file,
    );
    let mut stream = request.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        let len = bytes.len() as u64;
        writer.write_all(&bytes).await?;

        let prev = progress.fetch_add(len, Ordering::Relaxed);
        pb.set_position(prev + len);
    }

    writer.flush().await?;
    Ok(tmp_path.to_path_buf())
}

async fn merge_parts(
    output: &Path,
    parts: &[PathBuf],
    _total_size: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut final_file = fs::File::create(output).await?;

    let _ = final_file.set_len(_total_size).await;

    let mut buffer = vec![0u8; DEFAULT_BUFFER_SIZE];

    for (i, part) in parts.iter().enumerate() {
        log::debug!("Merging part {}: {}", i, part.display());
        let mut reader =
            tokio::io::BufReader::with_capacity(DEFAULT_BUFFER_SIZE, fs::File::open(part).await?);

        loop {
            let n = tokio::io::AsyncReadExt::read(&mut reader, &mut buffer).await?;
            if n == 0 {
                break;
            }
            tokio::io::AsyncWriteExt::write_all(&mut final_file, &buffer[..n]).await?;
        }

        fs::remove_file(part).await.ok();
    }

    final_file.sync_all().await.ok();
    log::info!("Merge complete: {}", output.display());
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

    download_optimized(
        &client,
        &format!("{}/file.txt", server.url("")),
        &output,
        &pb,
        false,
        11,
        DEFAULT_BUFFER_SIZE,
    )
    .await
    .unwrap();

    let content = tokio::fs::read(&output).await.unwrap();
    assert_eq!(content, body);
    m.assert();
    tokio::fs::remove_file(output).await.ok();
}

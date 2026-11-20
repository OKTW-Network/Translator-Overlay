//! Model file download into `models_dir`, shared by OCR and built-in translation models.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use thiserror::Error;
use tokio::{fs, io::AsyncWriteExt};
use tracing::info;
use xet::xet_session::{HeaderMap, XetConfig, XetFileInfo, XetSessionBuilder};

use crate::{ModelKind, PipelineStatus, http::http_client_builder, http_client};

/// Bounds connecting and each read, so a stalled transfer fails instead of freezing the progress bar.
const IDLE: Duration = Duration::from_secs(60);

/// One downloadable model file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelArtifact {
    pub file_name: &'static str,
    /// Download prefix that hosts the file.
    pub release: &'static str,
    /// The file's name under `release`.
    pub remote_name: &'static str,
    /// Exact byte length of the hosted file.
    pub expected_bytes: u64,
}

impl ModelArtifact {
    pub fn download_url(&self) -> String {
        format!("{}/{}", self.release, self.remote_name)
    }
}

#[derive(Debug, Error)]
#[error("model download: {0}")]
pub struct DownloadError(pub String);

/// Temporary file [`download_artifacts`] writes before the rename. A caller that drops the future
/// early deletes these.
pub fn part_path(models_dir: &Path, art: &ModelArtifact) -> PathBuf {
    // The process id keeps a crashed run's leftover from clashing with this one.
    models_dir.join(format!(".{}.{}.part", art.file_name, std::process::id()))
}

/// Download `arts` into `models_dir`, each through its own `.part` file.
///
/// Hugging Face files stored on Xet download together in one hf-xet batch (chunked, several files
/// at once); the rest go first as plain HTTPS GETs, one at a time. Each file's exact byte length
/// is the final integrity check. `on_status` gets [`PipelineStatus::DownloadingModels`] whenever
/// the finished-file count or the overall percent changes.
pub async fn download_artifacts(
    kind: ModelKind,
    arts: &[&ModelArtifact],
    models_dir: &Path,
    mut on_status: impl FnMut(PipelineStatus),
) -> Result<(), DownloadError> {
    if arts.is_empty() {
        return Ok(());
    }
    let total: u64 = arts.iter().map(|a| a.expected_bytes).sum();
    let mut last = None;
    // `done` files finished, `bytes` received of `total`.
    let mut report = |done: usize, bytes: u64| {
        let percent = (bytes.min(total) * 100 / total.max(1)) as u8;
        if last != Some((done, percent)) {
            last = Some((done, percent));
            on_status(PipelineStatus::DownloadingModels {
                kind,
                file_index: (done + 1).min(arts.len()) as u32,
                file_count: arts.len() as u32,
                percent,
            });
        }
    };
    report(0, 0);

    let client_err = |e: reqwest::Error| DownloadError(format!("http client: {e}"));
    let client = http_client(Some(IDLE)).map_err(client_err)?;
    let probe = http_client_builder(Some(IDLE))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(client_err)?;
    let mut http = Vec::new();
    let mut xet = Vec::new();
    let mut token_url = None;
    for &art in arts {
        match xet_source(&probe, &art.download_url()).await {
            // One read token covers one repo; a file from another repo takes the HTTP path.
            Some((hash, url)) if *token_url.get_or_insert_with(|| url.clone()) == url => xet.push((art, hash)),
            _ => http.push(art),
        }
    }

    let mut bytes = 0;
    for (done, &art) in http.iter().enumerate() {
        let part = part_path(models_dir, art);
        if let Err(e) = http_download(&client, art, &part, &mut |n| report(done, bytes + n)).await {
            let _ = fs::remove_file(&part).await;
            return Err(e);
        }
        commit_part(models_dir, art).await?;
        bytes += art.expected_bytes;
    }
    match token_url {
        Some(token_url) => {
            let report = &mut |done, n| report(http.len() + done, bytes + n);
            xet_download(&xet, token_url, models_dir, report).await
        }
        None => Ok(()),
    }
}

/// Xet hash and token route of a Hugging Face file stored on Xet, read from its `resolve`
/// redirect (`probe` must not follow redirects). `None` for any other file or a failed probe,
/// which then downloads over plain HTTP.
async fn xet_source(probe: &reqwest::Client, url: &str) -> Option<(String, String)> {
    let head = probe.head(url).send().await.ok()?;
    let hash = head.headers().get("x-xet-hash")?.to_str().ok()?;
    // `Link: <https://huggingface.co/api/models/…/xet-read-token/…>; rel="xet-auth", …`
    let link = head.headers().get(reqwest::header::LINK)?.to_str().ok()?;
    let auth = link.split(',').find(|l| l.contains(r#"rel="xet-auth""#))?;
    let token_url = auth.split_once('<')?.1.split_once('>')?.0;
    Some((hash.to_owned(), token_url.to_owned()))
}

/// Move a finished `.part` into place, removing it (or a wrong-sized result) on failure.
async fn commit_part(models_dir: &Path, art: &ModelArtifact) -> Result<(), DownloadError> {
    let part = part_path(models_dir, art);
    let dest = models_dir.join(art.file_name);
    if let Err(e) = fs::rename(&part, &dest).await {
        let _ = fs::remove_file(&part).await;
        return Err(DownloadError(format!("rename {} → {}: {e}", part.display(), dest.display())));
    }
    // Load only checks that the file exists, so a wrong-sized file here would never be fetched again.
    if !std::fs::metadata(&dest).is_ok_and(|m| m.is_file() && m.len() == art.expected_bytes) {
        let _ = fs::remove_file(&dest).await;
        return Err(DownloadError(format!("{}: size check failed after rename", art.file_name)));
    }
    info!(file = art.file_name, bytes = art.expected_bytes, "model download complete");
    Ok(())
}

/// Shuts the runtime down on drop, killing every task on it.
struct ShutdownOnDrop(Option<tokio::runtime::Runtime>);

impl Drop for ShutdownOnDrop {
    fn drop(&mut self) {
        if let Some(runtime) = self.0.take() {
            runtime.shutdown_background();
        }
    }
}

/// Download `files` (artifact, Xet hash) from the repo behind `token_url` in one hf-xet group,
/// then move each finished `.part` into place. `report` takes the same values as in
/// [`download_artifacts`], counted over `files` only.
async fn xet_download(
    files: &[(&ModelArtifact, String)],
    token_url: String,
    models_dir: &Path,
    report: &mut impl FnMut(usize, u64),
) -> Result<(), DownloadError> {
    let err = |e: xet::XetError| DownloadError(format!("xet: {e}"));
    let mut config = XetConfig::new();
    // No transfer reports to Hugging Face; the final one also holds `finish` up to 2 s.
    config.telemetry.enabled = false;
    // A stalled read retries after `IDLE` instead of the 300 s default.
    config.client.read_timeout = IDLE;
    // hf-xet fetches through detached `tokio::spawn`s that `XetSession::abort` never reaches, so
    // they run on a runtime of their own, shut down when this future ends or is dropped.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("hf-xet")
        .build()
        .map_err(|e| DownloadError(format!("xet runtime: {e}")))?;
    let handle = runtime.handle().clone();
    let _shutdown = ShutdownOnDrop(Some(runtime));
    // The chunk cache is off (`no-default-cache`); the shard cache stays out of `~/.cache/huggingface`.
    let session = XetSessionBuilder::new_with_config(config)
        .with_cache_dir(models_dir.join(".xet"))
        .with_tokio_handle(handle)
        .build()
        .map_err(err)?;
    let group = session
        .new_file_download_group()
        .map_err(err)?
        .with_token_refresh_url(token_url, HeaderMap::new())
        .build()
        .await
        .map_err(err)?;
    let mut handles = Vec::with_capacity(files.len());
    for (art, hash) in files {
        info!(file = art.file_name, url = %art.download_url(), expected = art.expected_bytes, "downloading model via xet");
        let info = XetFileInfo::new(hash.clone(), art.expected_bytes);
        handles.push(group.download_file_to_path(info, part_path(models_dir, art)).await.map_err(err)?);
    }

    let finish = group.clone().finish();
    tokio::pin!(finish);
    // The first tick fires at once.
    let mut tick = tokio::time::interval(Duration::from_millis(250));
    let result = loop {
        tokio::select! {
            result = &mut finish => break result,
            _ = tick.tick() => {
                let done = handles
                    .iter()
                    .filter(|h| h.progress().is_some_and(|p| p.total_bytes > 0 && p.bytes_completed >= p.total_bytes))
                    .count();
                // Written bytes only move near each file's end, so the bytes follow the
                // (compressed) network transfer instead. Both totals cover only the ranges planned
                // so far, which grow during the download.
                let p = group.progress();
                let bytes = u128::from(p.total_bytes) * u128::from(p.total_transfer_bytes_completed) / u128::from(p.total_transfer_bytes.max(1));
                report(done, bytes as u64);
            }
        }
    };

    // `finish` waits for every file, so after a failure the ones that made it are still kept.
    let mut result = result.map(drop).map_err(err);
    for ((art, _), handle) in files.iter().zip(&handles) {
        let kept = match handle.result() {
            Some(Ok(_)) => commit_part(models_dir, art).await,
            _ => {
                let _ = fs::remove_file(part_path(models_dir, art)).await;
                Ok(())
            }
        };
        result = result.and(kept);
    }
    result
}

async fn http_download(
    client: &reqwest::Client,
    art: &ModelArtifact,
    part: &Path,
    report: &mut impl FnMut(u64),
) -> Result<(), DownloadError> {
    let url = art.download_url();
    info!(file = art.file_name, %url, expected = art.expected_bytes, "downloading model");
    let mut response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| DownloadError(format!("{}: request failed: {e}", art.file_name)))?
        .error_for_status()
        .map_err(|e| DownloadError(format!("{}: {e}", art.file_name)))?;

    if let Some(len) = response.content_length()
        && len != art.expected_bytes
    {
        return Err(DownloadError(format!("{}: Content-Length {len} != expected {}", art.file_name, art.expected_bytes)));
    }

    let mut file = fs::File::create(part)
        .await
        .map_err(|e| DownloadError(format!("create {}: {e}", part.display())))?;
    let mut downloaded = 0_u64;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| DownloadError(format!("{}: stream: {e}", art.file_name)))?
    {
        file.write_all(&chunk)
            .await
            .map_err(|e| DownloadError(format!("{}: write: {e}", art.file_name)))?;
        downloaded = downloaded.saturating_add(chunk.len() as u64);
        if downloaded > art.expected_bytes {
            return Err(DownloadError(format!("{}: downloaded {downloaded} bytes > expected {}", art.file_name, art.expected_bytes)));
        }
        report(downloaded);
    }
    file.flush()
        .await
        .map_err(|e| DownloadError(format!("{}: flush: {e}", art.file_name)))?;
    if downloaded != art.expected_bytes {
        return Err(DownloadError(format!("{}: downloaded {downloaded} bytes != expected {}", art.file_name, art.expected_bytes)));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        io::{ErrorKind, Read, Write},
        net::TcpListener,
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        },
        time::Instant,
    };

    use tempfile::tempdir;

    use super::*;

    struct TestServer {
        base_url: String,
        expected: usize,
        served: Arc<AtomicUsize>,
        stop: Arc<AtomicBool>,
        handle: Option<std::thread::JoinHandle<()>>,
    }

    impl TestServer {
        fn spawn(responses: Vec<Vec<u8>>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let addr = listener.local_addr().unwrap();
            let expected = responses.len();
            let served = Arc::new(AtomicUsize::new(0));
            let served_srv = Arc::clone(&served);
            let stop = Arc::new(AtomicBool::new(false));
            let stop_srv = Arc::clone(&stop);

            let handle = std::thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(3);
                for resp in responses {
                    let mut stream = loop {
                        match listener.accept() {
                            Ok((s, _)) => break s,
                            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                                if stop_srv.load(Ordering::Acquire) || Instant::now() >= deadline {
                                    return;
                                }
                                std::thread::sleep(Duration::from_millis(5));
                            }
                            Err(_) => return,
                        }
                    };
                    let _ = stream.set_nonblocking(false);
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                    let mut buf = [0u8; 4096];
                    let mut read = 0;
                    while read < buf.len() {
                        let n = stream.read(&mut buf[read..]).unwrap_or(0);
                        if n == 0 {
                            break;
                        }
                        read += n;
                        if buf[..read].windows(4).any(|w| w == b"\r\n\r\n") {
                            break;
                        }
                    }
                    let _ = stream.write_all(&resp);
                    let _ = stream.flush();
                    served_srv.fetch_add(1, Ordering::Release);
                }
            });

            Self {
                base_url: format!("http://{addr}"),
                expected,
                served,
                stop,
                handle: Some(handle),
            }
        }

        fn finish(mut self) {
            if let Some(handle) = self.handle.take() {
                handle.join().unwrap();
            }
            assert_eq!(self.served.load(Ordering::Acquire), self.expected, "unexpected request count to TestServer");
        }
    }

    impl Drop for TestServer {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Release);
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        }
    }

    fn leaked_str(s: String) -> &'static str {
        Box::leak(s.into_boxed_str())
    }

    #[tokio::test]
    async fn empty_artifacts_succeeds_without_status_updates() {
        let dir = tempdir().unwrap();
        let mut updates = Vec::new();
        download_artifacts(ModelKind::Ocr, &[], dir.path(), |s| updates.push(s))
            .await
            .unwrap();
        assert!(updates.is_empty());
    }

    #[tokio::test]
    async fn commit_part_checks_size_and_cleans_up_on_mismatch() {
        let dir = tempdir().unwrap();
        let art = ModelArtifact {
            file_name: "model.onnx",
            release: "https://example.invalid",
            remote_name: "model.onnx",
            expected_bytes: 5,
        };
        let part = part_path(dir.path(), &art);
        let dest = dir.path().join(art.file_name);

        // Missing part returns error.
        assert!(commit_part(dir.path(), &art).await.is_err());

        // Exact size commits and replaces existing dest.
        std::fs::write(&dest, b"stale").unwrap();
        std::fs::write(&part, b"12345").unwrap();
        commit_part(dir.path(), &art).await.unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), b"12345");
        assert!(!part.exists());

        // Wrong size deletes dest so it cannot poison cache.
        std::fs::write(&part, b"123").unwrap();
        assert!(commit_part(dir.path(), &art).await.is_err());
        assert!(!dest.exists());
        assert!(!part.exists());
    }

    #[tokio::test]
    async fn http_download_commits_exact_bytes_and_reports_progress() {
        let dir = tempdir().unwrap();
        let body = b"hello world!".to_vec();
        let server = TestServer::spawn(vec![
            b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 12\r\n\r\n".to_vec(),
            [
                b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 12\r\n\r\n".as_slice(),
                &body,
            ]
            .concat(),
        ]);
        let art = ModelArtifact {
            file_name: "weights.bin",
            release: leaked_str(server.base_url.clone()),
            remote_name: "weights.bin",
            expected_bytes: 12,
        };

        let mut statuses = Vec::new();
        download_artifacts(ModelKind::Translation, &[&art], dir.path(), |s| statuses.push(s))
            .await
            .unwrap();
        server.finish();

        assert_eq!(std::fs::read(dir.path().join("weights.bin")).unwrap(), body);
        assert!(!part_path(dir.path(), &art).exists());
        assert!(matches!(
            statuses.first(),
            Some(PipelineStatus::DownloadingModels {
                kind: ModelKind::Translation,
                file_index: 1,
                file_count: 1,
                percent: 0,
            })
        ));
        assert!(matches!(
            statuses.last(),
            Some(PipelineStatus::DownloadingModels {
                kind: ModelKind::Translation,
                file_index: 1,
                file_count: 1,
                percent: 100,
            })
        ));
        // Progress updates are deduplicated.
        assert!(statuses.windows(2).all(|w| w[0] != w[1]));
    }

    #[tokio::test]
    async fn http_download_rejects_bad_status_and_size_mismatches() {
        let dir = tempdir().unwrap();
        // 1) Content-Length mismatch before opening `.part`.
        let s1 = TestServer::spawn(vec![
            b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 0\r\n\r\n".to_vec(),
            b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 99\r\n\r\n".to_vec(),
        ]);
        let art1 = ModelArtifact {
            file_name: "cl.bin",
            release: leaked_str(s1.base_url.clone()),
            remote_name: "cl.bin",
            expected_bytes: 5,
        };
        assert!(download_artifacts(ModelKind::Ocr, &[&art1], dir.path(), |_| {}).await.is_err());
        s1.finish();
        assert!(!part_path(dir.path(), &art1).exists());
        assert!(!dir.path().join("cl.bin").exists());

        // 2) Stream without Content-Length exceeding expected_bytes cleans up `.part`.
        let s2 = TestServer::spawn(vec![
            b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n".to_vec(),
            b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n123456789".to_vec(),
        ]);
        let art2 = ModelArtifact {
            file_name: "over.bin",
            release: leaked_str(s2.base_url.clone()),
            remote_name: "over.bin",
            expected_bytes: 5,
        };
        assert!(download_artifacts(ModelKind::Ocr, &[&art2], dir.path(), |_| {}).await.is_err());
        s2.finish();
        assert!(!part_path(dir.path(), &art2).exists());

        // 3) Truncated stream without Content-Length cleans up `.part`.
        let s3 = TestServer::spawn(vec![
            b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n".to_vec(),
            b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n12".to_vec(),
        ]);
        let art3 = ModelArtifact {
            file_name: "short.bin",
            release: leaked_str(s3.base_url.clone()),
            remote_name: "short.bin",
            expected_bytes: 5,
        };
        assert!(download_artifacts(ModelKind::Ocr, &[&art3], dir.path(), |_| {}).await.is_err());
        s3.finish();
        assert!(!part_path(dir.path(), &art3).exists());

        // 4) HTTP 404 errors cleanly.
        let s4 = TestServer::spawn(vec![
            b"HTTP/1.1 404 Not Found\r\nConnection: close\r\nContent-Length: 0\r\n\r\n".to_vec(),
            b"HTTP/1.1 404 Not Found\r\nConnection: close\r\nContent-Length: 0\r\n\r\n".to_vec(),
        ]);
        let art4 = ModelArtifact {
            file_name: "missing.bin",
            release: leaked_str(s4.base_url.clone()),
            remote_name: "missing.bin",
            expected_bytes: 5,
        };
        assert!(download_artifacts(ModelKind::Ocr, &[&art4], dir.path(), |_| {}).await.is_err());
        s4.finish();
        assert!(!dir.path().join("missing.bin").exists());
    }

    #[tokio::test]
    async fn xet_source_parses_headers_and_falls_back_when_absent() {
        let server = TestServer::spawn(vec![
            b"HTTP/1.1 302 Found\r\nConnection: close\r\nx-xet-hash: abc123\r\nLink: <https://example/other>; rel=\"canonical\", <https://example/tok>; rel=\"xet-auth\"\r\n\r\n".to_vec(),
            b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n".to_vec(),
        ]);
        let probe = http_client_builder(Some(Duration::from_secs(5)))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();

        let hit = xet_source(&probe, &format!("{}/a", server.base_url)).await;
        assert_eq!(hit, Some(("abc123".into(), "https://example/tok".into())));
        let miss = xet_source(&probe, &format!("{}/b", server.base_url)).await;
        assert_eq!(miss, None);
        server.finish();
    }
}

//! Model file download into `models_dir`, shared by OCR and built-in translation models.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use thiserror::Error;
use tokio::{fs, io::AsyncWriteExt};
use tracing::info;

use crate::http_client;

/// One downloadable model file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelArtifact {
    pub file_name: &'static str,
    /// Download prefix that hosts the file.
    pub release: &'static str,
    /// Exact byte length of the hosted file.
    pub expected_bytes: u64,
}

impl ModelArtifact {
    pub fn download_url(&self) -> String {
        format!("{}/{}", self.release, self.file_name)
    }
}

#[derive(Debug, Error)]
#[error("model download: {0}")]
pub struct DownloadError(pub String);

/// Client for model downloads, so a stalled transfer fails instead of freezing the progress bar.
pub fn download_client() -> reqwest::Result<reqwest::Client> {
    http_client(Some(Duration::from_secs(60)))
}

/// Temporary file [`download_artifact`] writes before the rename. A caller that drops the future early deletes it.
pub fn part_path(models_dir: &Path, art: &ModelArtifact) -> PathBuf {
    // The process id keeps a crashed run's leftover from clashing with this one.
    models_dir.join(format!(".{}.{}.part", art.file_name, std::process::id()))
}

/// Download one artifact into `models_dir` through a `.part` file.
///
/// HTTPS plus the exact byte length is the integrity check (no hash). `on_percent` fires once
/// per integer percent.
pub async fn download_artifact(
    client: &reqwest::Client,
    art: &ModelArtifact,
    models_dir: &Path,
    mut on_percent: impl FnMut(u8),
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

    let part = part_path(models_dir, art);
    let mut file = fs::File::create(&part)
        .await
        .map_err(|e| DownloadError(format!("create {}: {e}", part.display())))?;
    let written = async {
        let mut downloaded = 0_u64;
        let mut percent = 0;
        on_percent(percent);
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
            // Report only when the integer percent changes, not per chunk.
            let now = (downloaded * 100 / art.expected_bytes) as u8;
            if now != percent {
                percent = now;
                on_percent(percent);
            }
        }
        file.flush()
            .await
            .map_err(|e| DownloadError(format!("{}: flush: {e}", art.file_name)))?;
        if downloaded != art.expected_bytes {
            return Err(DownloadError(format!("{}: downloaded {downloaded} bytes != expected {}", art.file_name, art.expected_bytes)));
        }
        Ok(())
    }
    .await;
    drop(file);

    let dest = models_dir.join(art.file_name);
    let result = match written {
        Ok(()) => fs::rename(&part, &dest)
            .await
            .map_err(|e| DownloadError(format!("rename {} → {}: {e}", part.display(), dest.display()))),
        Err(e) => Err(e),
    };
    if let Err(e) = result {
        let _ = fs::remove_file(&part).await;
        return Err(e);
    }

    // Load only checks that the file exists, so a wrong-sized file here would never be fetched again.
    if !std::fs::metadata(&dest).is_ok_and(|m| m.is_file() && m.len() == art.expected_bytes) {
        let _ = fs::remove_file(&dest).await;
        return Err(DownloadError(format!("{}: size check failed after rename", art.file_name)));
    }

    info!(file = art.file_name, bytes = art.expected_bytes, "model download complete");
    Ok(())
}

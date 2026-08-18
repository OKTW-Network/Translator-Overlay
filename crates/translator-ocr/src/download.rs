//! PP-OCRv6 model download (GitHub Releases → `models_dir`) and load orchestration.

use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use tokio::{fs, io::AsyncWriteExt, sync::watch};
use tracing::{info, warn};
use translator_core::{ModelTier, OcrConfig, PipelineStatus};

use crate::{
    OcrEngine, OcrError,
    models::{ModelArtifact, ModelPaths, artifacts_for_tier, file_has_expected_size},
};

/// Progress for one file in an `ensure_models` run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadProgress {
    pub file_name: String,
    /// 0-based index among files that need download this run.
    pub file_index: u32,
    pub file_count: u32,
    pub bytes_downloaded: u64,
    pub bytes_total: u64,
}

impl DownloadProgress {
    pub fn percent(&self) -> u8 {
        if self.bytes_total == 0 {
            return 0;
        }
        let pct = (self.bytes_downloaded.saturating_mul(100)) / self.bytes_total;
        pct.min(100) as u8
    }
}

/// Latest download / ORT-load phase (`watch` coalesces to the newest value).
#[derive(Clone)]
pub enum ModelLoadUpdate {
    Downloading {
        file: String,
        /// 1-based index among files being fetched this run.
        file_index: u32,
        file_count: u32,
        percent: u8,
    },
    Loading,
    Ready(OcrEngine),
    Failed(String),
}

impl ModelLoadUpdate {
    /// UI status for in-progress phases (`None` once the load finished).
    pub fn to_status(&self) -> Option<PipelineStatus> {
        match self {
            Self::Downloading {
                file,
                file_index,
                file_count,
                percent,
            } => Some(PipelineStatus::DownloadingModels {
                file: file.clone(),
                file_index: *file_index,
                file_count: *file_count,
                percent: *percent,
            }),
            Self::Loading => Some(PipelineStatus::LoadingModels),
            Self::Ready(_) | Self::Failed(_) => None,
        }
    }
}

/// Handle for a background [`OcrEngine::start_load`] job.
pub struct ModelLoadTask {
    pub rx: watch::Receiver<ModelLoadUpdate>,
    pub tier: ModelTier,
}

impl OcrEngine {
    /// Download missing models (if needed) then build the ORT session in the background.
    ///
    /// Returns a [`ModelLoadTask`] whose `watch` receiver always holds the latest phase.
    /// Dropping the receiver ignores the result; the task may still finish in the background.
    pub fn start_load(config: OcrConfig) -> ModelLoadTask {
        let tier = config.model_tier;
        let initial = match config.models_dir_path() {
            Ok(dir) => {
                let missing = missing_artifacts(&dir, tier);
                if missing.is_empty() {
                    ModelLoadUpdate::Loading
                } else {
                    ModelLoadUpdate::Downloading {
                        file: missing[0].file_name.to_string(),
                        file_index: 1,
                        file_count: missing.len() as u32,
                        percent: 0,
                    }
                }
            }
            Err(e) => ModelLoadUpdate::Failed(e.to_string()),
        };

        let (tx, rx) = watch::channel(initial);
        tokio::spawn(async move {
            run_download_and_load(config, tx).await;
        });

        ModelLoadTask { rx, tier }
    }
}

async fn run_download_and_load(config: OcrConfig, tx: watch::Sender<ModelLoadUpdate>) {
    let tier = config.model_tier;
    let models_dir = match config.models_dir_path() {
        Ok(p) => p,
        Err(e) => {
            let _ = tx.send(ModelLoadUpdate::Failed(e.to_string()));
            return;
        }
    };

    // Publish only when the integer percent (or file index) changes — not per chunk.
    let mut last_published: Option<(u32, u8)> = None;
    let ensure = ensure_models(&models_dir, tier, |p: DownloadProgress| {
        let percent = p.percent();
        let key = (p.file_index, percent);
        if last_published == Some(key) {
            return;
        }
        last_published = Some(key);
        let _ = tx.send(ModelLoadUpdate::Downloading {
            file: p.file_name,
            file_index: p.file_index.saturating_add(1),
            file_count: p.file_count.max(1),
            percent,
        });
    })
    .await;

    if let Err(e) = ensure {
        let _ = tx.send(ModelLoadUpdate::Failed(e.to_string()));
        return;
    }

    let _ = tx.send(ModelLoadUpdate::Loading);
    match OcrEngine::load(&config).await {
        Ok(engine) => {
            let _ = tx.send(ModelLoadUpdate::Ready(engine));
        }
        Err(e) => {
            let _ = tx.send(ModelLoadUpdate::Failed(e.to_string()));
        }
    }
}

fn missing_artifacts(models_dir: &Path, tier: ModelTier) -> Vec<&'static ModelArtifact> {
    artifacts_for_tier(tier)
        .iter()
        .filter(|a| {
            let path = models_dir.join(a.file_name);
            !file_has_expected_size(&path, a.expected_bytes)
        })
        .collect()
}

/// Ensure all artifacts for `tier` exist under `models_dir` with the expected size.
///
/// Existing files with the correct byte length are skipped. Truncated / wrong-sized
/// files are re-downloaded. HTTPS + exact size is the integrity check (no hash).
pub async fn ensure_models(
    models_dir: &Path,
    tier: ModelTier,
    mut on_progress: impl FnMut(DownloadProgress),
) -> Result<ModelPaths, OcrError> {
    fs::create_dir_all(models_dir)
        .await
        .map_err(|e| OcrError::Other(format!("create models dir {}: {e}", models_dir.display())))?;

    let paths = ModelPaths::from_dir(models_dir, tier);
    let missing = missing_artifacts(models_dir, tier);

    if missing.is_empty() {
        return Ok(paths);
    }

    let client = reqwest::Client::builder()
        .user_agent(concat!("translator-overlay/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| OcrError::Download(format!("http client: {e}")))?;

    let file_count = missing.len() as u32;
    for (file_index, art) in missing.into_iter().enumerate() {
        let dest = models_dir.join(art.file_name);
        download_one(&client, art, &dest, file_index as u32, file_count, &mut on_progress).await?;
    }

    if !paths.all_present() {
        return Err(OcrError::Download("models still incomplete after download".into()));
    }
    Ok(paths)
}

async fn download_one(
    client: &reqwest::Client,
    art: &ModelArtifact,
    dest: &Path,
    file_index: u32,
    file_count: u32,
    on_progress: &mut impl FnMut(DownloadProgress),
) -> Result<(), OcrError> {
    if file_has_expected_size(dest, art.expected_bytes) {
        return Ok(());
    }
    if fs::try_exists(dest).await.unwrap_or(false) {
        warn!(
            path = %dest.display(),
            expected = art.expected_bytes,
            "removing wrong-sized OCR model before re-download"
        );
        let _ = fs::remove_file(dest).await;
    }

    let url = art.download_url();
    info!(file = art.file_name, %url, expected = art.expected_bytes, "downloading OCR model");

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| OcrError::Download(format!("{}: request failed: {e}", art.file_name)))?
        .error_for_status()
        .map_err(|e| OcrError::Download(format!("{}: {e}", art.file_name)))?;

    if let Some(len) = response.content_length()
        && len != art.expected_bytes
    {
        return Err(OcrError::Download(format!("{}: Content-Length {len} != expected {}", art.file_name, art.expected_bytes)));
    }

    let part_path = part_path_for(dest);
    if let Some(parent) = part_path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| OcrError::Download(format!("create part dir: {e}")))?;
    }

    let result = write_download_part(response, art, &part_path, dest, file_index, file_count, on_progress).await;
    if result.is_err() {
        let _ = fs::remove_file(&part_path).await;
    }
    result
}

async fn write_download_part(
    response: reqwest::Response,
    art: &ModelArtifact,
    part_path: &Path,
    dest: &Path,
    file_index: u32,
    file_count: u32,
    on_progress: &mut impl FnMut(DownloadProgress),
) -> Result<(), OcrError> {
    let mut file = fs::File::create(part_path)
        .await
        .map_err(|e| OcrError::Download(format!("create {}: {e}", part_path.display())))?;

    let mut downloaded = 0_u64;
    let mut stream = response.bytes_stream();
    on_progress(DownloadProgress {
        file_name: art.file_name.to_string(),
        file_index,
        file_count,
        bytes_downloaded: 0,
        bytes_total: art.expected_bytes,
    });

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| OcrError::Download(format!("{}: stream: {e}", art.file_name)))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| OcrError::Download(format!("{}: write: {e}", art.file_name)))?;
        downloaded = downloaded.saturating_add(chunk.len() as u64);
        if downloaded > art.expected_bytes {
            return Err(OcrError::Download(format!("{}: downloaded {downloaded} bytes > expected {}", art.file_name, art.expected_bytes)));
        }
        on_progress(DownloadProgress {
            file_name: art.file_name.to_string(),
            file_index,
            file_count,
            bytes_downloaded: downloaded,
            bytes_total: art.expected_bytes,
        });
    }

    file.flush()
        .await
        .map_err(|e| OcrError::Download(format!("{}: flush: {e}", art.file_name)))?;
    drop(file);

    if downloaded != art.expected_bytes {
        return Err(OcrError::Download(format!("{}: downloaded {downloaded} bytes != expected {}", art.file_name, art.expected_bytes)));
    }

    fs::rename(part_path, dest)
        .await
        .map_err(|e| OcrError::Download(format!("rename {} → {}: {e}", part_path.display(), dest.display())))?;

    if !file_has_expected_size(dest, art.expected_bytes) {
        let _ = fs::remove_file(dest).await;
        return Err(OcrError::Download(format!("{}: size check failed after rename", art.file_name)));
    }

    info!(file = art.file_name, bytes = downloaded, "OCR model download complete");
    Ok(())
}

fn part_path_for(dest: &Path) -> PathBuf {
    let name = dest.file_name().and_then(|s| s.to_str()).unwrap_or("model");
    let pid = std::process::id();
    dest.with_file_name(format!(".{name}.{pid}.part"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_caps_at_100() {
        let p = DownloadProgress {
            file_name: "x".into(),
            file_index: 0,
            file_count: 1,
            bytes_downloaded: 50,
            bytes_total: 100,
        };
        assert_eq!(p.percent(), 50);
        let done = DownloadProgress {
            bytes_downloaded: 100,
            ..p.clone()
        };
        assert_eq!(done.percent(), 100);
    }

    #[test]
    fn part_path_is_hidden_sibling() {
        let dest = Path::new("models/pp-ocrv6_small_det.onnx");
        let part = part_path_for(dest);
        let name = part.file_name().unwrap().to_string_lossy();
        assert!(name.starts_with(".pp-ocrv6_small_det.onnx."));
        assert!(name.ends_with(".part"));
    }
}

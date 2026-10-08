//! Download PP-OCRv6 models from GitHub Releases into `models_dir`, then load them.

use std::path::Path;

use tokio::{fs, io::AsyncWriteExt, sync::watch};
use tracing::info;
use translator_core::{ModelTier, OcrConfig, OcrDevice, PipelineStatus};

use crate::{
    OcrEngine, OcrError,
    models::{ModelArtifact, artifacts_for_tier},
};

/// Latest phase of a download and ORT load. The `watch` channel keeps only the newest value.
#[derive(Clone)]
pub enum ModelLoadUpdate {
    /// `DownloadingModels` or `LoadingModels`, ready to show as the pipeline status.
    Progress(PipelineStatus),
    Ready(OcrEngine),
    Failed(String),
}

/// Handle for a background [`OcrEngine::start_load`] job.
pub struct ModelLoadTask {
    pub rx: watch::Receiver<ModelLoadUpdate>,
    pub tier: ModelTier,
    pub device: OcrDevice,
}

impl OcrEngine {
    /// In the background, download any missing models and then build the ORT session.
    ///
    /// The returned [`ModelLoadTask`] has a `watch` receiver that always holds the latest
    /// phase. Dropping the receiver ignores the result, though the task may still finish.
    pub fn start_load(config: OcrConfig) -> ModelLoadTask {
        let tier = config.model_tier;
        let device = config.device;
        let initial = match config.models_dir_path() {
            Ok(dir) => {
                let missing = missing_artifacts(&dir, tier);
                ModelLoadUpdate::Progress(match missing.first() {
                    Some(first) => downloading(first.file_name, 0, missing.len(), 0),
                    None => PipelineStatus::LoadingModels,
                })
            }
            Err(e) => ModelLoadUpdate::Failed(e.to_string()),
        };

        let (tx, rx) = watch::channel(initial);
        tokio::spawn(async move {
            let update = match download_and_load(&config, &tx).await {
                Ok(engine) => ModelLoadUpdate::Ready(engine),
                Err(e) => ModelLoadUpdate::Failed(e.to_string()),
            };
            let _ = tx.send(update);
        });

        ModelLoadTask { rx, tier, device }
    }
}

/// `DownloadingModels` for file `index` (0-based) of `count`.
fn downloading(file: &str, index: usize, count: usize, percent: u8) -> PipelineStatus {
    PipelineStatus::DownloadingModels {
        file: file.to_string(),
        file_index: index as u32 + 1,
        file_count: count as u32,
        percent,
    }
}

async fn download_and_load(config: &OcrConfig, tx: &watch::Sender<ModelLoadUpdate>) -> Result<OcrEngine, OcrError> {
    let models_dir = config.models_dir_path()?;
    let missing = missing_artifacts(&models_dir, config.model_tier);
    if !missing.is_empty() {
        fs::create_dir_all(&models_dir)
            .await
            .map_err(|e| OcrError::Other(format!("create models dir {}: {e}", models_dir.display())))?;
        let client = reqwest::Client::builder()
            .user_agent(concat!("translator-overlay/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| OcrError::Download(format!("http client: {e}")))?;
        for (index, art) in missing.iter().enumerate() {
            // Publish only when the integer percent changes, not per chunk.
            let mut last_percent = None;
            download_one(&client, art, &models_dir, |percent| {
                if last_percent != Some(percent) {
                    last_percent = Some(percent);
                    let _ = tx.send(ModelLoadUpdate::Progress(downloading(art.file_name, index, missing.len(), percent)));
                }
            })
            .await?;
        }
    }
    let _ = tx.send(ModelLoadUpdate::Progress(PipelineStatus::LoadingModels));
    OcrEngine::load(config).await
}

fn missing_artifacts(models_dir: &Path, tier: ModelTier) -> Vec<&'static ModelArtifact> {
    artifacts_for_tier(tier)
        .iter()
        .filter(|a| !models_dir.join(a.file_name).is_file())
        .collect()
}

/// Download one artifact into `models_dir` through a `.part` file.
///
/// HTTPS and the exact byte length are the only integrity checks. There is no hash.
async fn download_one(
    client: &reqwest::Client,
    art: &ModelArtifact,
    models_dir: &Path,
    mut on_percent: impl FnMut(u8),
) -> Result<(), OcrError> {
    let url = art.download_url();
    info!(file = art.file_name, %url, expected = art.expected_bytes, "downloading OCR model");

    let mut response = client
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

    // The process id keeps a crashed run's leftover from clashing with this one.
    let part = models_dir.join(format!(".{}.{}.part", art.file_name, std::process::id()));
    let mut file = fs::File::create(&part)
        .await
        .map_err(|e| OcrError::Download(format!("create {}: {e}", part.display())))?;
    let written = async {
        let mut downloaded = 0_u64;
        on_percent(0);
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| OcrError::Download(format!("{}: stream: {e}", art.file_name)))?
        {
            file.write_all(&chunk)
                .await
                .map_err(|e| OcrError::Download(format!("{}: write: {e}", art.file_name)))?;
            downloaded = downloaded.saturating_add(chunk.len() as u64);
            if downloaded > art.expected_bytes {
                return Err(OcrError::Download(format!(
                    "{}: downloaded {downloaded} bytes > expected {}",
                    art.file_name, art.expected_bytes
                )));
            }
            on_percent((downloaded * 100 / art.expected_bytes) as u8);
        }
        file.flush()
            .await
            .map_err(|e| OcrError::Download(format!("{}: flush: {e}", art.file_name)))?;
        if downloaded != art.expected_bytes {
            return Err(OcrError::Download(format!("{}: downloaded {downloaded} bytes != expected {}", art.file_name, art.expected_bytes)));
        }
        Ok(())
    }
    .await;
    drop(file);

    let dest = models_dir.join(art.file_name);
    let result = match written {
        Ok(()) => fs::rename(&part, &dest)
            .await
            .map_err(|e| OcrError::Download(format!("rename {} → {}: {e}", part.display(), dest.display()))),
        Err(e) => Err(e),
    };
    if result.is_err() {
        let _ = fs::remove_file(&part).await;
    } else {
        info!(file = art.file_name, bytes = art.expected_bytes, "OCR model download complete");
    }
    result
}

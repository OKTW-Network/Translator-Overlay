//! PP-OCRv6 / PP-DocLayoutV3 model download (GitHub Releases → `models_dir`) and load orchestration.

use std::path::{Path, PathBuf};

use tokio::{fs, io::AsyncWriteExt, sync::watch, task::JoinHandle};
use tracing::info;
use translator_core::{ModelKind, ModelTier, OcrConfig, OcrDevice, PipelineStatus};

use crate::{
    OcrEngine, OcrError,
    models::{LAYOUT_ARTIFACT, ModelArtifact, artifacts_for_tier},
};

/// Latest download / ORT-load phase (`watch` coalesces to the newest value).
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
    /// Built with PP-DocLayoutV3 ([`OcrConfig::uses_layout`]).
    pub layout: bool,
    join: JoinHandle<()>,
}

impl ModelLoadTask {
    /// Abandon a pending download, then wait for the task to end. A finished or in-progress load
    /// is dropped inside the task, so its ORT sessions are gone when this returns.
    pub async fn close(self) {
        drop(self.rx);
        let _ = self.join.await;
    }
}

impl OcrEngine {
    /// Download missing models (if needed) then build the ORT session in the background.
    ///
    /// Returns a [`ModelLoadTask`] whose `watch` receiver always holds the latest phase.
    /// Dropping the receiver abandons a pending download; an ORT load still runs to the end.
    pub fn start_load(config: OcrConfig) -> ModelLoadTask {
        let tier = config.model_tier;
        let device = config.device;
        let layout = config.uses_layout();
        let initial = match config.models_dir_path() {
            Ok(dir) => {
                let missing = missing_artifacts(&dir, &config);
                ModelLoadUpdate::Progress(match missing.first() {
                    Some(first) => downloading(first.file_name, 0, missing.len(), 0),
                    None => PipelineStatus::LoadingModels { kind: ModelKind::Ocr },
                })
            }
            Err(e) => ModelLoadUpdate::Failed(e.to_string()),
        };

        let (tx, rx) = watch::channel(initial);
        let join = tokio::spawn(async move {
            let update = match download_and_load(&config, &tx).await {
                Ok(engine) => ModelLoadUpdate::Ready(engine),
                Err(e) => ModelLoadUpdate::Failed(e.to_string()),
            };
            let _ = tx.send(update);
        });

        ModelLoadTask {
            rx,
            tier,
            device,
            layout,
            join,
        }
    }
}

/// `DownloadingModels` for file `index` (0-based) of `count`.
fn downloading(file: &str, index: usize, count: usize, percent: u8) -> PipelineStatus {
    PipelineStatus::DownloadingModels {
        kind: ModelKind::Ocr,
        file: file.to_string(),
        file_index: index as u32 + 1,
        file_count: count as u32,
        percent,
    }
}

async fn download_and_load(config: &OcrConfig, tx: &watch::Sender<ModelLoadUpdate>) -> Result<OcrEngine, OcrError> {
    let models_dir = config.models_dir_path()?;
    let missing = missing_artifacts(&models_dir, config);
    if !missing.is_empty() {
        fs::create_dir_all(&models_dir)
            .await
            .map_err(|e| OcrError::Other(format!("create models dir {}: {e}", models_dir.display())))?;
        let client = reqwest::Client::builder()
            .user_agent(concat!("translator-overlay/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| OcrError::Download(format!("http client: {e}")))?;
        for (index, art) in missing.iter().enumerate() {
            let download = download_artifact(&client, art, &models_dir, |percent| {
                let _ = tx.send(ModelLoadUpdate::Progress(downloading(art.file_name, index, missing.len(), percent)));
            });
            tokio::select! {
                biased;
                () = tx.closed() => {
                    let _ = fs::remove_file(part_path(&models_dir, art)).await;
                    return Err(OcrError::Other("OCR model load abandoned".into()));
                }
                result = download => result?,
            }
        }
    }
    // Abandoned: skip the load. Once started it is not interrupted ([`ModelLoadTask::close`] waits for it).
    if tx.is_closed() {
        return Err(OcrError::Other("OCR model load abandoned".into()));
    }
    let _ = tx.send(ModelLoadUpdate::Progress(PipelineStatus::LoadingModels { kind: ModelKind::Ocr }));
    OcrEngine::load(config).await
}

fn missing_artifacts(models_dir: &Path, config: &OcrConfig) -> Vec<&'static ModelArtifact> {
    let layout = config.uses_layout().then_some(&LAYOUT_ARTIFACT);
    artifacts_for_tier(config.model_tier)
        .iter()
        .chain(layout)
        .filter(|a| !models_dir.join(a.file_name).is_file())
        .collect()
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
) -> Result<(), OcrError> {
    let url = art.download_url();
    info!(file = art.file_name, %url, expected = art.expected_bytes, "downloading model");

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

    let part = part_path(models_dir, art);
    let mut file = fs::File::create(&part)
        .await
        .map_err(|e| OcrError::Download(format!("create {}: {e}", part.display())))?;
    let written = async {
        let mut downloaded = 0_u64;
        let mut percent = 0;
        on_percent(percent);
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
            // Report only when the integer percent changes, not per chunk.
            let now = (downloaded * 100 / art.expected_bytes) as u8;
            if now != percent {
                percent = now;
                on_percent(percent);
            }
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
    if let Err(e) = result {
        let _ = fs::remove_file(&part).await;
        return Err(e);
    }

    // Load only checks that the file exists, so a wrong-sized file here would never be fetched again.
    if !std::fs::metadata(&dest).is_ok_and(|m| m.is_file() && m.len() == art.expected_bytes) {
        let _ = fs::remove_file(&dest).await;
        return Err(OcrError::Download(format!("{}: size check failed after rename", art.file_name)));
    }

    info!(file = art.file_name, bytes = art.expected_bytes, "model download complete");
    Ok(())
}

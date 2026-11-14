//! PP-OCRv6 / PP-DocLayoutV3 model download (Hugging Face / GitHub Releases → `models_dir`) and load orchestration.

use std::path::Path;

use tokio::{fs, sync::watch, task::JoinHandle};
use translator_core::{
    ModelArtifact, ModelKind, ModelTier, OcrConfig, OcrDevice, OcrModel, PipelineStatus, V5Det, download_artifacts, part_path,
};

use crate::{
    OcrEngine, OcrError,
    models::{LAYOUT_ARTIFACT, artifacts},
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
    pub model: OcrModel,
    pub v5_det: V5Det,
    pub device: OcrDevice,
    /// Built with PP-DocLayoutV3 ([`OcrConfig::uses_layout`]).
    pub layout: bool,
    pub det_max_side: u32,
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
        let model = config.model;
        let v5_det = config.v5_det;
        let device = config.device;
        let layout = config.uses_layout();
        let det_max_side = config.det_max_side;
        let initial = match config.models_dir_path() {
            Ok(dir) => {
                let missing = missing_artifacts(&dir, &config);
                ModelLoadUpdate::Progress(if missing.is_empty() {
                    PipelineStatus::LoadingModels { kind: ModelKind::Ocr }
                } else {
                    PipelineStatus::DownloadingModels {
                        kind: ModelKind::Ocr,
                        file_index: 1,
                        file_count: missing.len() as u32,
                        percent: 0,
                    }
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
            model,
            v5_det,
            device,
            layout,
            det_max_side,
            join,
        }
    }
}

async fn download_and_load(config: &OcrConfig, tx: &watch::Sender<ModelLoadUpdate>) -> Result<OcrEngine, OcrError> {
    let models_dir = config.models_dir_path()?;
    let missing = missing_artifacts(&models_dir, config);
    if !missing.is_empty() {
        fs::create_dir_all(&models_dir)
            .await
            .map_err(|e| OcrError::Other(format!("create models dir {}: {e}", models_dir.display())))?;
        let download = download_artifacts(ModelKind::Ocr, &missing, &models_dir, |status| {
            let _ = tx.send(ModelLoadUpdate::Progress(status));
        });
        tokio::select! {
            biased;
            () = tx.closed() => {
                for art in &missing {
                    let _ = fs::remove_file(part_path(&models_dir, art)).await;
                }
                return Err(OcrError::Other("OCR model load abandoned".into()));
            }
            result = download => result?,
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
    artifacts(config.model_tier, config.model, config.v5_det)
        .into_iter()
        .chain(layout)
        .filter(|a| !models_dir.join(a.file_name).is_file())
        .collect()
}

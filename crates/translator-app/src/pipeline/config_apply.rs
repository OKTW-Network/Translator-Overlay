//! Config apply and OCR engine load.

use std::path::PathBuf;

use tracing::{error, info, warn};
use translator_capture::CapturedFrame;
use translator_core::{AppConfig, OcrConfig};
use translator_ocr::{BlockPersistenceFilter, OcrEngine, StabilityGate, prepare_engine};

use crate::pipeline::worker::Pipeline;

impl Pipeline {
    pub(crate) fn apply_config(&mut self, cfg: AppConfig) {
        // Model tier change requires a full engine reload (new ONNX weights).
        let engine_reload = self.ocr_tier != cfg.ocr.model_tier;
        self.show_preview = cfg.capture.show_preview;
        self.gate = StabilityGate::from_config(&cfg.ocr);
        self.persist = BlockPersistenceFilter::from_config(&cfg.ocr);
        self.client.update_api(cfg.api.clone());

        if let Some(o) = self.overlay.as_ref() {
            let _ = o.update_config(cfg.overlay.clone());
        }

        if let Err(e) = cfg.save_default_path() {
            error!(error = %e, "failed to save config");
            let mut s = self.state.write();
            s.config = cfg;
            s.set_error(format!("save config: {e}"));
            return;
        }

        {
            let mut s = self.state.write();
            s.config = cfg.clone();
            // Single short line for the UI InfoBar title (avoid title+message pair).
            s.settings_message = Some("Saved".into());
            s.last_error = None;
        }
        info!("config applied and saved");

        if engine_reload {
            // Reload weights / ORT session for the new tier.
            self.ocr_tier = cfg.ocr.model_tier;
            self.engine = None;
            match load_engine(&self.state, &cfg.ocr) {
                Ok(e) => self.engine = Some(e),
                Err(e) => {
                    self.state.write().set_error(format!("models: {e}"));
                    return;
                }
            }
        } else if let Some(eng) = self.engine.as_mut() {
            // Same engine: still pick up confidence / line-merge / filter knobs.
            eng.apply_runtime_config(&cfg.ocr);
        }

        // Restore a sensible non-error status after save.
        self.state.write().restore_operational_status();
    }

    pub(crate) fn ensure_engine(&mut self) -> bool {
        if self.engine.is_some() {
            return true;
        }
        let cfg = self.state.read().config.ocr.clone();
        match load_engine(&self.state, &cfg) {
            Ok(e) => {
                self.engine = Some(e);
                true
            }
            Err(e) => {
                self.state.write().set_error(format!("OCR: {e}"));
                false
            }
        }
    }

    pub(crate) fn update_preview(&self, frame: &CapturedFrame) {
        let mut s = self.state.write();
        s.frame_count = frame.sequence;
        s.preview.width = frame.width;
        s.preview.height = frame.height;
        s.preview.sequence = frame.sequence;

        if self.show_preview {
            match save_preview(frame) {
                Ok(path) => s.preview.path = Some(path.display().to_string()),
                Err(e) => warn!(error = %e, "failed to save preview"),
            }
        }
    }
}

pub(crate) fn load_engine(state: &crate::pipeline::SharedState, cfg: &OcrConfig) -> Result<OcrEngine, translator_ocr::OcrError> {
    state.write().status = translator_core::PipelineStatus::LoadingModels;
    // May block while oar-ocr fetches missing registry files / loads ORT.
    prepare_engine(cfg)
}

fn save_preview(frame: &CapturedFrame) -> Result<PathBuf, String> {
    let thumb = frame.thumbnail(480);
    let png = thumb.to_png_bytes().map_err(|e| e.to_string())?;
    let dir = translator_core::exe_dir().map_err(|e| e.to_string())?;
    let path = dir.join("preview_last.png");
    std::fs::write(&path, png).map_err(|e| e.to_string())?;
    Ok(path)
}

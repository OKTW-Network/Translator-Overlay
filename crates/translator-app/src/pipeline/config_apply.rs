//! Config apply and OCR engine load.

use tracing::{error, info};
use translator_capture::CapturedFrame;
use translator_core::{AppConfig, PipelineStatus};
use translator_ocr::{BlockPersistenceFilter, OcrEngine, StabilityGate};

use crate::pipeline::worker::Pipeline;

impl Pipeline {
    /// Persist only overlay / reader visibility on the live config.
    pub(crate) fn set_overlay_display(&mut self, enabled: bool, reader_enabled: bool) {
        let overlay = {
            let mut s = self.state.write();
            s.config.overlay.enabled = enabled;
            s.config.overlay.reader_enabled = reader_enabled;
            s.config.overlay.clone()
        };
        if let Some(o) = self.overlay.as_ref() {
            let _ = o.update_config(overlay);
        }
        let save = self.state.read().config.clone();
        if let Err(e) = save.save_default_path() {
            error!(error = %e, "failed to save overlay display flags");
            self.state.write().set_error(format!("save config: {e}"));
            return;
        }
        info!(enabled, reader_enabled, "overlay display updated");
    }

    pub(crate) async fn apply_config(&mut self, cfg: AppConfig) {
        // Model tier change requires a full engine reload (new ONNX weights).
        let engine_reload = self.ocr_tier != cfg.ocr.model_tier;
        self.gate = StabilityGate::from_config(&cfg.ocr);
        self.persist = BlockPersistenceFilter::from_config(&cfg.ocr);
        self.client.update_api(cfg.api.clone());
        self.translation_cache.set_max(cfg.translation.cache_max_entries_clamped());

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
            s.translation_cache_len = self.translation_cache.len();
            // Single short line for the UI InfoBar title (avoid title+message pair).
            s.settings_message = Some("Saved".into());
            s.last_error = None;
        }
        info!("config applied and saved");

        if engine_reload {
            // Reload weights / ORT session for the new tier.
            self.ocr_tier = cfg.ocr.model_tier;
            self.engine = None;
            self.state.write().status = PipelineStatus::LoadingModels;
            match OcrEngine::load(&cfg.ocr).await {
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

    pub(crate) async fn ensure_engine(&mut self) -> bool {
        if self.engine.is_some() {
            return true;
        }
        let cfg = self.state.read().config.ocr.clone();
        self.state.write().status = PipelineStatus::LoadingModels;
        match OcrEngine::load(&cfg).await {
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
        s.preview.rgba = Some(frame.rgba.clone());
    }
}

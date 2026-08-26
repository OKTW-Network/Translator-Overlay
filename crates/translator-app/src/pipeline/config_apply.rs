//! Config apply and OCR engine load wiring.

use tracing::{error, info};
use translator_core::{AppConfig, PipelineStatus};
use translator_ocr::{BlockPersistenceFilter, ModelLoadUpdate, OcrEngine, StabilityGate};

use crate::pipeline::worker::{InflightModelLoad, Pipeline};

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
        let engine_reload = self.ocr_tier != cfg.ocr.model_tier;
        self.gate = StabilityGate::from_config(&cfg.ocr);
        self.persist = BlockPersistenceFilter::from_config(&cfg.ocr);
        let identity_changed = {
            let prev = self.client.api();
            prev.model != cfg.api.model || prev.http_api != cfg.api.http_api || prev.provider != cfg.api.provider
        };
        if identity_changed {
            self.cancel_inflight();
            self.conversation.clear();
        }
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
            s.settings_message = Some("Saved".into());
            s.last_error = None;
        }
        info!("config applied and saved");

        if engine_reload {
            self.ocr_tier = cfg.ocr.model_tier;
            self.engine = None;
            self.start_model_load();
        } else if let Some(eng) = self.engine.as_mut() {
            eng.apply_runtime_config(&cfg.ocr);
            self.state.write().restore_operational_status();
        } else if self.model_load.is_none() {
            // Engine missing and no load in flight (e.g. prior failure) — retry.
            self.start_model_load();
        } else {
            // Keep Downloading/Loading status; do not wipe via restore_operational_status.
            self.sync_model_load();
        }
    }

    /// `true` when the OCR engine is ready. Starts a background load if needed.
    pub(crate) fn ensure_engine(&mut self) -> bool {
        if self.engine.is_some() {
            return true;
        }
        if self.model_load.is_none() {
            self.start_model_load();
        }
        false
    }

    /// Start a background OCR download/load if one is not already running.
    ///
    /// Does not cancel or replace an in-flight job. If a job for another tier is
    /// already running, it is left alone; [`sync_model_load`] starts the desired
    /// tier once that job finishes.
    pub(crate) fn start_model_load(&mut self) {
        let desired = self.state.read().config.ocr.model_tier;
        if let Some(job) = self.model_load.as_ref() {
            if job.tier == desired {
                info!(tier = ?desired, "OCR model load already in progress");
            } else {
                info!(
                    in_flight = ?job.tier,
                    desired = ?desired,
                    "OCR model load already in progress for another tier; will restart after it finishes"
                );
            }
            self.sync_model_load();
            return;
        }

        let task = OcrEngine::start_load(self.state.read().config.ocr.clone());
        self.ocr_tier = task.tier;
        self.model_load = Some(InflightModelLoad {
            rx: task.rx,
            tier: task.tier,
        });
        info!(tier = ?task.tier, "OCR model load started");
        self.sync_model_load();
    }

    /// Apply the latest `watch` value from `translator-ocr`.
    pub(crate) fn sync_model_load(&mut self) {
        let Some(job) = self.model_load.as_mut() else {
            return;
        };
        let sender_gone = job.rx.has_changed().is_err();
        let update = job.rx.borrow_and_update().clone();
        let tier = job.tier;
        let desired = self.state.read().config.ocr.model_tier;

        match update {
            ModelLoadUpdate::Ready(engine) => {
                self.model_load = None;
                if tier != desired {
                    info!(finished = ?tier, desired = ?desired, "discarding OCR engine for stale tier");
                    self.start_model_load();
                    return;
                }
                info!(?tier, "OCR engine ready");
                self.ocr_tier = tier;
                self.engine = Some(engine);
                if let Some(eng) = self.engine.as_mut() {
                    eng.apply_runtime_config(&self.state.read().config.ocr);
                }
                self.state.write().restore_operational_status();
            }
            ModelLoadUpdate::Failed(message) => {
                self.model_load = None;
                if tier != desired {
                    info!(
                        finished = ?tier,
                        desired = ?desired,
                        error = %message,
                        "stale-tier OCR load failed; starting desired tier"
                    );
                    self.start_model_load();
                    return;
                }
                error!(error = %message, ?tier, "OCR model load failed");
                self.state.write().set_error(format!("models: {message}"));
            }
            progress if sender_gone => {
                self.model_load = None;
                if tier != desired {
                    info!(finished = ?tier, desired = ?desired, "stale-tier OCR load ended unexpectedly; starting desired tier");
                    self.start_model_load();
                    return;
                }
                error!(?tier, status = ?progress.to_status(), "OCR model load task ended unexpectedly");
                self.state.write().set_error("models: load task ended unexpectedly");
            }
            progress => {
                self.state.write().status = progress.to_status().unwrap_or(PipelineStatus::LoadingModels);
            }
        }
    }
}

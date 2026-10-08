//! Config apply and OCR engine load wiring.

use std::path::PathBuf;

use rust_i18n::t;
use tracing::{error, info};
use translator_core::{AppConfig, PipelineStatus, UiLanguage, config_path};
use translator_ocr::{BlockPersistenceFilter, ModelLoadUpdate, OcrEngine, StabilityGate};
use translator_overlay::OverlayCommand;

use crate::pipeline::worker::{InflightModelLoad, Pipeline};

impl Pipeline {
    /// Resolve the default config path; on failure record `save config:` and return `None`.
    fn default_config_path(&mut self) -> Option<PathBuf> {
        match config_path() {
            Ok(p) => Some(p),
            Err(e) => {
                error!(error = %e, "failed to resolve config path");
                self.state.write().set_error(t!("err.save_config", error = e.to_string()));
                None
            }
        }
    }

    /// Persist only overlay / reader / HUD visibility on the live config.
    pub(crate) fn set_overlay_display(&mut self, enabled: bool, reader_enabled: bool, hud_enabled: bool) {
        let overlay = {
            let mut s = self.state.write();
            s.config.overlay.enabled = enabled;
            s.config.overlay.reader_enabled = reader_enabled;
            s.config.overlay.hud_enabled = hud_enabled;
            s.config.overlay.clone()
        };
        if let Some(o) = self.overlay.as_ref() {
            let _ = o.send(OverlayCommand::UpdateConfig(overlay));
        }
        if !self.persist_live_config() {
            return;
        }
        info!(enabled, reader_enabled, hud_enabled, "overlay display updated");
    }

    /// Send the translation-window placeholder in the current UI language.
    pub(crate) fn sync_reader_placeholder(&self) {
        if let Some(o) = self.overlay.as_ref() {
            let _ = o.send(OverlayCommand::SetReaderPlaceholder(t!("reader.placeholder").into_owned()));
        }
    }

    /// Persist only the control-window language on the live config.
    pub(crate) fn set_ui_language(&mut self, language: UiLanguage) {
        self.state.write().config.ui.language = Some(language);
        self.sync_reader_placeholder();
        if !self.persist_live_config() {
            return;
        }
        info!(language = language.as_str(), "ui language updated");
    }

    fn persist_live_config(&mut self) -> bool {
        let save = self.state.read().config.clone();
        let Some(path) = self.default_config_path() else {
            return false;
        };
        if let Err(e) = save.save(&path) {
            error!(error = %e, "failed to save live config");
            self.state.write().set_error(t!("err.save_config", error = e.to_string()));
            return false;
        }
        true
    }

    pub(crate) async fn apply_config(&mut self, cfg: AppConfig) {
        // Threshold / filter / merge knobs change how raw blocks are derived.
        self.last_raw_ocr = None;
        let engine_reload = self.ocr_tier != cfg.ocr.model_tier || self.ocr_device != cfg.ocr.device;
        self.gate = StabilityGate::from_config(&cfg.ocr);
        self.persist = BlockPersistenceFilter::from_config(&cfg.ocr);
        let identity_changed = {
            let prev = self.client.config();
            prev.model != cfg.api.model || prev.http_api != cfg.api.http_api || prev.provider != cfg.api.provider
        };
        if identity_changed {
            self.cancel_inflight();
            self.conversation.clear();
        }
        self.client.update_config(cfg.api.clone());
        self.translation_cache.set_max(cfg.translation.cache_max_entries_clamped());

        if let Some(o) = self.overlay.as_ref() {
            let _ = o.send(OverlayCommand::UpdateConfig(cfg.overlay.clone()));
        }
        // Reload from disk can change the UI language.
        self.sync_reader_placeholder();

        self.state.write().config = cfg.clone();
        let Some(path) = self.default_config_path() else {
            return;
        };
        if let Err(e) = cfg.save(&path) {
            error!(error = %e, "failed to save config");
            let mut s = self.state.write();
            s.config = cfg;
            s.set_error(t!("err.save_config", error = e.to_string()));
            return;
        }

        {
            let mut s = self.state.write();
            s.config = cfg.clone();
            s.translation_cache_len = self.translation_cache.len();
            s.settings_message = Some(translator_core::SETTINGS_SAVED.into());
            s.last_error = None;
        }
        info!("config applied and saved");

        if engine_reload {
            self.ocr_tier = cfg.ocr.model_tier;
            self.ocr_device = cfg.ocr.device;
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
    /// Does not cancel or replace an in-flight job. If a job for another engine
    /// identity is already running, it is left alone; [`sync_model_load`] starts
    /// the desired load once that job finishes.
    pub(crate) fn start_model_load(&mut self) {
        let desired = self.state.read().config.ocr.clone();
        if let Some(job) = self.model_load.as_ref() {
            if (job.tier, job.device) == (desired.model_tier, desired.device) {
                info!(tier = ?desired.model_tier, device = ?desired.device, "OCR model load already in progress");
            } else {
                info!(
                    in_flight = ?(job.tier, job.device),
                    desired = ?(desired.model_tier, desired.device),
                    "OCR model load already in progress for another engine; will restart after it finishes"
                );
            }
            self.sync_model_load();
            return;
        }

        let task = OcrEngine::start_load(desired);
        self.ocr_tier = task.tier;
        self.ocr_device = task.device;
        self.model_load = Some(InflightModelLoad {
            rx: task.rx,
            tier: task.tier,
            device: task.device,
        });
        info!(tier = ?task.tier, device = ?task.device, "OCR model load started");
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
        let device = job.device;
        let desired = self.state.read().config.ocr.clone();

        match update {
            ModelLoadUpdate::Ready(engine) => {
                self.model_load = None;
                if (tier, device) != (desired.model_tier, desired.device) {
                    info!(
                        finished = ?(tier, device),
                        desired = ?(desired.model_tier, desired.device),
                        "discarding OCR engine for stale identity"
                    );
                    self.start_model_load();
                    return;
                }
                info!(?tier, ?device, "OCR engine ready");
                self.ocr_tier = tier;
                self.ocr_device = device;
                self.engine = Some(engine);
                if let Some(eng) = self.engine.as_mut() {
                    eng.apply_runtime_config(&self.state.read().config.ocr);
                }
                self.state.write().restore_operational_status();
            }
            ModelLoadUpdate::Failed(message) => {
                self.model_load = None;
                if (tier, device) != (desired.model_tier, desired.device) {
                    info!(
                        finished = ?(tier, device),
                        desired = ?(desired.model_tier, desired.device),
                        error = %message,
                        "stale OCR load failed; starting desired engine"
                    );
                    self.start_model_load();
                    return;
                }
                error!(error = %message, ?tier, ?device, "OCR model load failed");
                self.state.write().set_error(t!("err.models", error = message));
            }
            progress if sender_gone => {
                self.model_load = None;
                if (tier, device) != (desired.model_tier, desired.device) {
                    info!(
                        finished = ?(tier, device),
                        desired = ?(desired.model_tier, desired.device),
                        "stale OCR load ended unexpectedly; starting desired engine"
                    );
                    self.start_model_load();
                    return;
                }
                error!(?tier, ?device, status = ?progress.to_status(), "OCR model load task ended unexpectedly");
                self.state.write().set_error(t!("err.models_task_ended"));
            }
            progress => {
                self.state.write().status = progress.to_status().unwrap_or(PipelineStatus::LoadingModels);
            }
        }
    }
}

//! Config apply and OCR engine load wiring.

use std::path::PathBuf;

use rust_i18n::t;
use tracing::{error, info};
use translator_core::{AppConfig, AppState, ModelProvider, OcrDevice, UiLanguage, config_path, load_active_dictionary};
use translator_ocr::{BlockPersistenceFilter, ModelLoadUpdate, OcrEngine, StabilityGate};
use translator_overlay::OverlayCommand;

use crate::pipeline::worker::Pipeline;

pub(crate) fn apply_dictionary_profile(state: &mut AppState) {
    match load_active_dictionary(state.config.translation.dictionary_profile.as_deref()) {
        Ok(pairs) => state.dictionary_pairs = pairs,
        Err(e) => {
            state.dictionary_pairs.clear();
            state.set_error(t!("err.load_dictionary", error = e.to_string()));
        }
    }
}

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
        let engine_reload =
            (self.ocr_tier, self.ocr_device, self.ocr_layout) != (cfg.ocr.model_tier, cfg.ocr.device, cfg.ocr.uses_layout());
        self.gate = StabilityGate::from_config(&cfg.ocr);
        self.persist = BlockPersistenceFilter::from_config(&cfg.ocr);
        if self.session.set_min_interval(cfg.capture.frame_interval()) {
            self.absorb_preview_sequence();
        }
        let prev = self.client.config();
        let builtin_changed = prev.provider != cfg.api.provider
            || prev.builtin_model != cfg.api.builtin_model
            || prev.builtin_device != cfg.api.builtin_device;
        let identity_changed = builtin_changed || prev.model != cfg.api.model || prev.http_api != cfg.api.http_api;
        // ORT's WebGPU EP frees dropped weights only when a Conv PrePack (an OCR load) flushes them,
        // or with the context both models share. A new OCR engine cleans up after itself; a new
        // built-in model does not, so drop both and let the loads below start a fresh context.
        let shared_webgpu =
            self.ocr_device == OcrDevice::Webgpu && prev.provider == ModelProvider::Builtin && prev.builtin_device == OcrDevice::Webgpu;
        if identity_changed {
            self.cancel_inflight();
            self.conversation.clear();
        }
        if shared_webgpu && builtin_changed && self.state.read().auto_running {
            // The unload would otherwise wait for an in-flight turn to finish.
            self.cancel_inflight();
            self.cancel_preload();
            self.release_models().await;
        }
        self.client.update_config(cfg.api.clone());
        if let Ok(dir) = cfg.ocr.models_dir_path() {
            self.client.set_models_dir(dir);
        }
        if identity_changed {
            self.cancel_preload();
            if self.state.read().auto_running {
                self.start_preload();
            }
        }
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
            self.state.write().set_error(t!("err.save_config", error = e.to_string()));
            return;
        }

        let old_pairs = self.state.read().dictionary_pairs.clone();
        let glossary_changed = {
            let mut s = self.state.write();
            s.settings_message = Some(translator_core::SETTINGS_SAVED.into());
            s.last_error = None;
            apply_dictionary_profile(&mut s);
            s.dictionary_pairs != old_pairs
        };
        if glossary_changed {
            self.cancel_inflight();
            self.conversation.clear();
            self.client.reset_session();
            self.translation_cache.clear();
            self.last_translated_fp = None;
        }
        self.state.write().translation_cache_len = self.translation_cache.len();
        info!("config applied and saved");

        if engine_reload {
            self.ocr_tier = cfg.ocr.model_tier;
            self.ocr_device = cfg.ocr.device;
            self.ocr_layout = cfg.ocr.uses_layout();
            self.engine = None;
        }
        if let Some(eng) = self.engine.as_mut() {
            eng.apply_runtime_config(&cfg.ocr);
            self.state.write().restore_operational_status();
        } else if self.model_load.is_some() {
            // Keep Downloading/Loading status; a stale engine restarts once its load ends.
            self.sync_model_load();
        } else if self.state.read().auto_running {
            // Engine reload, or a prior failure — load now.
            self.start_model_load();
        } else {
            // Idle: models load on the next Start.
            self.state.write().restore_operational_status();
        }
    }

    /// `true` when the OCR engine is ready. Starts a background load if needed while capturing.
    pub(crate) fn ensure_engine(&mut self) -> bool {
        if self.engine.is_some() {
            return true;
        }
        if self.model_load.is_none() && self.state.read().auto_running {
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
            if (job.tier, job.device, job.layout) == (desired.model_tier, desired.device, desired.uses_layout()) {
                info!(tier = ?desired.model_tier, device = ?desired.device, layout = job.layout, "OCR model load already in progress");
            } else {
                info!(
                    in_flight = ?(job.tier, job.device, job.layout),
                    desired = ?(desired.model_tier, desired.device, desired.uses_layout()),
                    "OCR model load already in progress for another engine; will restart after it finishes"
                );
            }
            self.sync_model_load();
            return;
        }

        let task = OcrEngine::start_load(desired);
        self.ocr_tier = task.tier;
        self.ocr_device = task.device;
        self.ocr_layout = task.layout;
        info!(tier = ?task.tier, device = ?task.device, layout = task.layout, "OCR model load started");
        self.model_load = Some(task);
        self.sync_model_load();
    }

    /// Apply the latest `watch` value from `translator-ocr`.
    pub(crate) fn sync_model_load(&mut self) {
        let Some(job) = self.model_load.as_mut() else {
            return;
        };
        let sender_gone = job.rx.has_changed().is_err();
        let (tier, device, layout) = (job.tier, job.device, job.layout);
        let finished = match job.rx.borrow_and_update().clone() {
            ModelLoadUpdate::Progress(status) if !sender_gone => {
                self.state.write().status = status;
                return;
            }
            finished => finished,
        };

        // The job is done, or its task ended without a result.
        self.model_load = None;
        let desired = self.state.read().config.ocr.clone();
        if (tier, device, layout) != (desired.model_tier, desired.device, desired.uses_layout()) {
            info!(
                finished = ?(tier, device, layout),
                desired = ?(desired.model_tier, desired.device, desired.uses_layout()),
                "OCR load ended for a stale engine; starting the desired one"
            );
            self.start_model_load();
            return;
        }
        match finished {
            ModelLoadUpdate::Ready(mut engine) => {
                info!(?tier, ?device, "OCR engine ready");
                engine.apply_runtime_config(&desired);
                self.engine = Some(engine);
                self.state.write().restore_operational_status();
            }
            ModelLoadUpdate::Failed(message) => {
                error!(error = %message, ?tier, ?device, "OCR model load failed");
                self.state.write().set_error(t!("err.models", error = message));
            }
            ModelLoadUpdate::Progress(status) => {
                error!(?tier, ?device, ?status, "OCR model load task ended unexpectedly");
                self.state.write().set_error(t!("err.models_task_ended"));
            }
        }
    }
}

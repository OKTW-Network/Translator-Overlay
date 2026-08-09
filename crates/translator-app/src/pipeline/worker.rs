//! Pipeline worker: owns capture / OCR / translate state and the command loop.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use parking_lot::RwLock;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use translator_capture::{CaptureSession, CapturedFrame};
use translator_core::{AppState, ModelTier, OcrBlock, PipelineStatus};
use translator_ocr::{BlockPersistenceFilter, OcrEngine, OcrFingerprint, StabilityGate};
use translator_overlay::OverlayController;
use translator_translate::{Conversation, TranslateClient, TranslateError};

use crate::pipeline::{PipelineCommand, config_apply::load_engine};

pub type SharedState = Arc<RwLock<AppState>>;
pub type CmdTx = std::sync::mpsc::Sender<PipelineCommand>;
pub type CmdRx = std::sync::mpsc::Receiver<PipelineCommand>;

/// Result of a background translate HTTP job.
pub(crate) struct TranslateJobResult {
    pub result: Result<String, TranslateError>,
    /// Conversation messages length after pushing the user turn (for rollback).
    pub messages_len_after_user: usize,
}

pub(crate) struct InflightTranslate {
    pub cancel: CancellationToken,
    pub rx: oneshot::Receiver<TranslateJobResult>,
    pub fingerprint: OcrFingerprint,
    pub blocks: Vec<OcrBlock>,
    pub source_text: String,
    pub content_width: u32,
    pub content_height: u32,
    /// Conversation length after the user turn was pushed (rollback on cancel/drop).
    pub messages_len_after_user: usize,
}

#[derive(Clone)]
pub(crate) struct PendingPage {
    pub blocks: Vec<OcrBlock>,
    pub source_text: String,
    pub fingerprint: OcrFingerprint,
    pub content_width: u32,
    pub content_height: u32,
}

/// Owned pipeline state machine (one instance per background thread).
pub(crate) struct Pipeline {
    pub state: SharedState,
    pub session: CaptureSession,
    pub show_preview: bool,
    pub gate: StabilityGate,
    pub persist: BlockPersistenceFilter,
    pub engine: Option<OcrEngine>,
    pub conversation: Conversation,
    pub client: TranslateClient,
    pub last_translated_fp: Option<OcrFingerprint>,
    pub last_page: Option<PendingPage>,
    pub inflight: Option<InflightTranslate>,
    pub ocr_tier: ModelTier,
    /// Wall-clock: raw OCR first went empty (may still have hysteresis tracks).
    pub raw_empty_since: Option<Instant>,
    /// Wall-clock since raw OCR last became non-empty.
    pub raw_content_since: Option<Instant>,
    /// Sticky remap failed while captions still present.
    pub remap_miss_since: Option<Instant>,
    /// Last WGC frame (static windows often stop delivering samples).
    pub last_frame: Option<CapturedFrame>,
    pub last_ocr_at: Option<Instant>,
    pub overlay: Option<OverlayController>,
    pub rt: tokio::runtime::Runtime,
}

pub fn spawn_pipeline(state: SharedState, rx: CmdRx) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("pipeline".into())
        .spawn(move || Pipeline::new(state).run(rx))
        .expect("spawn pipeline thread")
}

impl Pipeline {
    fn new(state: SharedState) -> Self {
        let show_preview = state.read().config.capture.show_preview;
        let ocr_cfg = state.read().config.ocr.clone();
        let ocr_tier = ocr_cfg.model_tier;
        let client = TranslateClient::new(state.read().config.api.clone());
        let overlay = match OverlayController::spawn(state.read().config.overlay.clone()) {
            Ok(o) => Some(o),
            Err(e) => {
                error!(error = %e, "failed to start overlay host");
                None
            }
        };
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");

        let mut pipeline = Self {
            state,
            session: CaptureSession::new(),
            show_preview,
            gate: StabilityGate::from_config(&ocr_cfg),
            persist: BlockPersistenceFilter::from_config(&ocr_cfg),
            engine: None,
            conversation: Conversation::new(),
            client,
            last_translated_fp: None,
            last_page: None,
            inflight: None,
            ocr_tier,
            raw_empty_since: None,
            raw_content_since: None,
            remap_miss_since: None,
            last_frame: None,
            last_ocr_at: None,
            overlay,
            rt,
        };

        // Load OCR engine on start (oar-ocr may auto-download missing models).
        let cfg = pipeline.state.read().config.ocr.clone();
        match load_engine(&pipeline.state, &cfg) {
            Ok(e) => pipeline.engine = Some(e),
            Err(e) => {
                error!(error = %e, "initial model load failed");
                pipeline.state.write().set_error(format!("models: {e}"));
            }
        }

        pipeline
    }

    fn run(mut self, rx: CmdRx) {
        loop {
            while let Ok(cmd) = rx.try_recv() {
                if self.handle_command(cmd) {
                    return;
                }
            }

            self.poll_translate();
            self.tick_capture();
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Returns `true` when the worker should shut down.
    fn handle_command(&mut self, cmd: PipelineCommand) -> bool {
        match cmd {
            PipelineCommand::Shutdown => {
                self.cancel_inflight();
                self.session.stop();
                if let Some(mut o) = self.overlay.take() {
                    let _ = o.clear();
                    o.shutdown();
                }
                info!("pipeline shutdown");
                return true;
            }
            PipelineCommand::CancelTranslate => {
                if let Some(job) = self.inflight.as_ref() {
                    info!("cancelling in-flight translation");
                    job.cancel.cancel();
                } else {
                    let mut s = self.state.write();
                    s.translate_in_flight = false;
                    if s.auto_running {
                        s.status = PipelineStatus::Capturing;
                    } else {
                        s.status = PipelineStatus::Cancelled;
                    }
                }
            }
            PipelineCommand::RetryTranslate => {
                if self.inflight.is_some() {
                    warn!("retry ignored — translation already in flight");
                } else if let Some(page) = self.last_page.clone() {
                    // Force re-translate even if content fingerprint matches.
                    self.last_translated_fp = None;
                    self.start_translate(page, true);
                } else {
                    self.state.write().set_error("nothing to retry".to_string());
                }
            }
            PipelineCommand::ApplyConfig(cfg) => self.apply_config(*cfg),
            PipelineCommand::StopCapture => self.stop_capture(),
            PipelineCommand::StartForeground => {
                let interval = self.state.read().config.capture.min_interval_ms;
                self.cancel_inflight();
                match self.session.start_foreground(interval) {
                    Ok(()) => self.on_capture_started(),
                    Err(e) => {
                        error!(error = %e, "start foreground capture failed");
                        self.state.write().set_error(e.to_string());
                    }
                }
            }
            PipelineCommand::StartCapture { hwnd, title } => {
                let interval = self.state.read().config.capture.min_interval_ms;
                self.cancel_inflight();
                match self.session.start_window(hwnd, title, interval) {
                    Ok(()) => self.on_capture_started(),
                    Err(e) => {
                        error!(error = %e, "start capture failed");
                        self.state.write().set_error(e.to_string());
                    }
                }
            }
            PipelineCommand::ManualCapture => self.manual_capture(),
            PipelineCommand::SetShowPreview(v) => {
                self.show_preview = v;
                self.state.write().config.capture.show_preview = v;
                let _ = self.state.read().config.save_default_path();
            }
            PipelineCommand::ResetConversation => {
                self.conversation.clear();
                self.last_translated_fp = None;
                info!("LLM conversation reset");
                self.state.write().restore_operational_status();
            }
        }
        false
    }

    fn cancel_inflight(&mut self) {
        if let Some(job) = self.inflight.take() {
            job.cancel.cancel();
            // Drop the pending user turn so stop/shutdown mid-request does not
            // leave a dangling multi-user conversation for the next translate.
            self.conversation.rollback_user_turn(job.messages_len_after_user);
        }
    }

    /// Shared reset when a continuous capture session successfully starts.
    fn on_capture_started(&mut self) {
        self.reset_ocr_session(true);
        if let Some(o) = self.overlay.as_ref() {
            let _ = o.clear();
            if let Some(hwnd) = self.session.target_hwnd {
                let _ = o.attach(hwnd);
            }
        }
        let mut s = self.state.write();
        s.auto_running = true;
        s.target_window_title = self.session.target_title.clone();
        s.target_hwnd = self.session.target_hwnd;
        s.translate_in_flight = false;
        s.last_error = None;
        s.status = PipelineStatus::Capturing;
    }

    fn stop_capture(&mut self) {
        self.cancel_inflight();
        self.session.stop();
        self.reset_ocr_session(false);
        if let Some(o) = self.overlay.as_ref() {
            let _ = o.detach();
            let _ = o.clear();
        }
        let mut s = self.state.write();
        s.auto_running = false;
        s.target_hwnd = None;
        s.translate_in_flight = false;
        s.status = PipelineStatus::Idle;
    }

    /// Reset gate / timers / last frame / captions. Optionally clear LLM conversation.
    fn reset_ocr_session(&mut self, clear_conversation: bool) {
        self.gate.reset_all();
        self.persist.reset();
        if clear_conversation {
            self.conversation.clear();
        }
        self.last_translated_fp = None;
        self.last_page = None;
        self.raw_empty_since = None;
        self.raw_content_since = None;
        self.remap_miss_since = None;
        self.last_frame = None;
        self.last_ocr_at = None;
        // Drop OCR + caption state so sticky remap cannot resurrect a prior session.
        let mut s = self.state.write();
        s.latest_ocr_blocks.clear();
        s.latest_ocr_text.clear();
        s.latest_translated_blocks.clear();
        s.latest_translated_text.clear();
        s.can_retry_translate = false;
        s.translate_in_flight = false;
    }

    fn manual_capture(&mut self) {
        if self.inflight.is_some() {
            warn!("manual capture ignored — translation in flight (cancel first)");
            return;
        }
        if !self.ensure_engine() {
            return;
        }

        // One-shot: attach overlay to the window we are about to capture.
        let one_shot = !self.session.is_running();
        let frame = if one_shot {
            let interval = self.state.read().config.capture.min_interval_ms;
            if let Err(e) = self.session.start_foreground(interval) {
                self.state.write().set_error(e.to_string());
                return;
            }
            if let Some(o) = self.overlay.as_ref()
                && let Some(hwnd) = self.session.target_hwnd
            {
                let _ = o.attach(hwnd);
            }
            {
                let mut s = self.state.write();
                s.target_window_title = self.session.target_title.clone();
                s.target_hwnd = self.session.target_hwnd;
            }
            let f = self.session.recv_frame_timeout(Duration::from_secs(2));
            // Keep target for overlay; stop only the capture stream.
            self.session.stop_stream_keep_target();
            f
        } else {
            self.session
                .take_latest_frame()
                .or_else(|| self.session.recv_frame_timeout(Duration::from_millis(500)))
        };

        if let Some(frame) = frame {
            self.update_preview(&frame);
            self.run_ocr_manual(&frame);
        } else {
            self.state.write().set_error("manual capture timeout");
        }
    }

    fn tick_capture(&mut self) {
        // Capture + OCR while not translating (OCR stays off the UI thread).
        if !self.session.is_running() || self.inflight.is_some() {
            return;
        }

        let interval_ms = self.state.read().config.capture.min_interval_ms.max(50);
        let stale_due = self
            .last_ocr_at
            .map(|t| t.elapsed() >= Duration::from_millis(interval_ms))
            .unwrap_or(true);

        // New WGC frames always OCR immediately. Fully static windows often
        // stop delivering frames entirely — re-OCR the last sample on the
        // capture interval so block persistence + stability still advance
        // and translation can fire without waiting for a screen change.
        let frame_for_ocr = match self.session.take_latest_frame() {
            Some(frame) => {
                self.last_frame = Some(frame.clone());
                Some(frame)
            }
            None if stale_due => self.last_frame.clone(),
            None => None,
        };

        if let Some(frame) = frame_for_ocr {
            self.last_ocr_at = Some(Instant::now());
            self.update_preview(&frame);

            if self.engine.is_none() {
                let mut s = self.state.write();
                if !matches!(
                    s.status,
                    PipelineStatus::Error { .. }
                        | PipelineStatus::LoadingModels
                        | PipelineStatus::Translating
                        | PipelineStatus::OverlayActive
                        | PipelineStatus::Cancelled
                ) {
                    s.status = PipelineStatus::Capturing;
                }
            } else {
                self.run_ocr_auto(&frame);
            }
        } else {
            // Between OCR ticks: still expire captions after raw-empty grace
            // so blank screens do not keep the last translation forever.
            self.maybe_expire_raw_empty();
        }
    }
}

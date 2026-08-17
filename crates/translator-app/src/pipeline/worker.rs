//! Pipeline worker: owns capture / OCR / translate state and the command loop.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use parking_lot::RwLock;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use translator_capture::CaptureSession;
use translator_core::{AppState, ModelTier, OcrBlock, PipelineStatus};
use translator_ocr::{BlockPersistenceFilter, OcrEngine, OcrFingerprint, StabilityGate};
use translator_overlay::{OverlayController, OverlayEvent};
use translator_translate::{Conversation, TranslateClient, TranslateError, TranslationCache};

use crate::pipeline::PipelineCommand;

pub type SharedState = Arc<RwLock<AppState>>;
pub type CmdRx = mpsc::UnboundedReceiver<PipelineCommand>;

/// UI → pipeline command sender (safe to call from the WinUI thread).
#[derive(Clone, Debug)]
pub struct CmdTx {
    tx: mpsc::UnboundedSender<PipelineCommand>,
}

impl CmdTx {
    pub fn send(&self, cmd: PipelineCommand) -> Result<(), mpsc::error::SendError<PipelineCommand>> {
        self.tx.send(cmd)
    }
}

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
    /// Unique miss blocks actually sent to the model (ids preserved).
    pub miss_blocks: Vec<OcrBlock>,
    /// Per-source-block cache hits (`None` = wait for the model / fallback).
    pub cached_hits: Vec<Option<String>>,
}

#[derive(Clone)]
pub(crate) struct PendingPage {
    pub blocks: Vec<OcrBlock>,
    pub source_text: String,
    pub fingerprint: OcrFingerprint,
    pub content_width: u32,
    pub content_height: u32,
}

/// Owned pipeline state machine (one Tokio task).
pub(crate) struct Pipeline {
    pub state: SharedState,
    pub session: CaptureSession,
    pub gate: StabilityGate,
    pub persist: BlockPersistenceFilter,
    pub engine: Option<OcrEngine>,
    pub conversation: Conversation,
    pub translation_cache: TranslationCache,
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
    pub overlay: Option<OverlayController>,
}

pub fn spawn_pipeline(state: SharedState) -> (CmdTx, tokio::task::JoinHandle<()>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let cmd_tx = CmdTx { tx };
    let join = tokio::spawn(async move {
        let mut pipeline = Pipeline::new(state).await;
        pipeline.run(rx).await;
    });
    (cmd_tx, join)
}

impl Pipeline {
    async fn new(state: SharedState) -> Self {
        let ocr_cfg = state.read().config.ocr.clone();
        let ocr_tier = ocr_cfg.model_tier;
        let client = TranslateClient::new(state.read().config.api.clone());
        let cache_max = state.read().config.translation.cache_max_entries_clamped();
        let overlay_cfg = state.read().config.overlay.clone();
        let overlay = match OverlayController::spawn(overlay_cfg).await {
            Ok(o) => Some(o),
            Err(e) => {
                error!(error = %e, "failed to start overlay host");
                None
            }
        };

        let mut pipeline = Self {
            state,
            session: CaptureSession::new(),
            gate: StabilityGate::from_config(&ocr_cfg),
            persist: BlockPersistenceFilter::from_config(&ocr_cfg),
            engine: None,
            conversation: Conversation::new(),
            translation_cache: TranslationCache::new(cache_max),
            client,
            last_translated_fp: None,
            last_page: None,
            inflight: None,
            ocr_tier,
            raw_empty_since: None,
            raw_content_since: None,
            remap_miss_since: None,
            overlay,
        };

        // Load OCR engine on start (oar-ocr may auto-download missing models).
        let cfg = pipeline.state.read().config.ocr.clone();
        pipeline.state.write().status = PipelineStatus::LoadingModels;
        match OcrEngine::load(&cfg).await {
            Ok(e) => pipeline.engine = Some(e),
            Err(e) => {
                error!(error = %e, "initial model load failed");
                pipeline.state.write().set_error(format!("models: {e}"));
            }
        }

        pipeline
    }

    async fn run(&mut self, mut rx: CmdRx) {
        loop {
            while let Ok(cmd) = rx.try_recv() {
                if self.handle_command(cmd).await {
                    return;
                }
            }

            self.drive_capture().await;
            let wait = self.next_wait();
            match next_event(&mut rx, self.inflight.as_mut(), self.overlay.as_mut(), wait).await {
                PipelineEvent::Command(None) => return,
                PipelineEvent::Command(Some(cmd)) => {
                    if self.handle_command(cmd).await {
                        return;
                    }
                }
                PipelineEvent::Translate(result) => {
                    let job = self.inflight.take().expect("inflight present");
                    let job_result = result.unwrap_or_else(|_| TranslateJobResult {
                        result: Err(TranslateError::Other("translate task dropped".into())),
                        messages_len_after_user: job.messages_len_after_user,
                    });
                    self.finish_translate(job, job_result);
                }
                PipelineEvent::Overlay(None) => {
                    warn!("overlay host event channel closed");
                    self.overlay = None;
                }
                PipelineEvent::Overlay(Some(ev)) => {
                    self.apply_overlay_event(ev);
                    while let Some(ev) = self.overlay.as_mut().and_then(OverlayController::try_recv_event) {
                        self.apply_overlay_event(ev);
                    }
                }
                PipelineEvent::Tick => {}
            }
        }
    }

    fn next_wait(&self) -> Option<Duration> {
        if self.session.is_running() && self.inflight.is_none() {
            let ms = self.state.read().config.capture.min_interval_ms.max(50);
            Some(Duration::from_millis(ms))
        } else {
            None
        }
    }

    /// Returns `true` when the worker should shut down.
    async fn handle_command(&mut self, cmd: PipelineCommand) -> bool {
        match cmd {
            PipelineCommand::Shutdown => {
                self.cancel_inflight();
                self.session.stop();
                if let Some(mut o) = self.overlay.take() {
                    let _ = o.clear();
                    o.shutdown().await;
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
            PipelineCommand::ApplyConfig(cfg) => self.apply_config(*cfg).await,
            PipelineCommand::SetOverlayDisplay { enabled, reader_enabled } => self.set_overlay_display(enabled, reader_enabled),
            PipelineCommand::StopCapture => self.stop_capture(),
            PipelineCommand::BeginRegionSelect { hwnd } => self.begin_region_select(hwnd),
            PipelineCommand::CancelRegionSelect => {
                if let Some(o) = self.overlay.as_ref() {
                    let _ = o.cancel_region_select();
                }
            }
            PipelineCommand::ConfirmRegionSelect => {
                if let Some(o) = self.overlay.as_ref() {
                    let _ = o.confirm_region_select();
                }
            }
            PipelineCommand::ClearRegionSelect => {
                if let Some(o) = self.overlay.as_ref() {
                    let _ = o.clear_region_select();
                }
                let mut s = self.state.write();
                s.region_select_draft.clear();
            }
            PipelineCommand::SetCaptureRegions { regions } => {
                if let Some(o) = self.overlay.as_ref() {
                    let _ = o.cancel_region_select();
                }
                self.apply_ocr_regions(regions);
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
            PipelineCommand::ManualCapture => self.manual_capture().await,
            PipelineCommand::ResetConversation => {
                self.conversation.clear();
                self.client.reset_session();
                self.last_translated_fp = None;
                info!("LLM conversation reset");
                self.state.write().restore_operational_status();
            }
            PipelineCommand::ClearTranslationCache => {
                self.translation_cache.clear();
                let mut s = self.state.write();
                s.translation_cache_len = 0;
                info!("translation cache cleared");
                s.restore_operational_status();
            }
        }
        false
    }

    pub(crate) fn cancel_inflight(&mut self) {
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
            if let Some(hwnd) = self.session.target_hwnd() {
                let _ = o.attach(hwnd);
            }
        }
        let mut s = self.state.write();
        s.auto_running = true;
        s.target_window_title = self.session.target_title();
        s.target_hwnd = self.session.target_hwnd();
        s.translate_in_flight = false;
        s.last_error = None;
        s.status = PipelineStatus::Capturing;
    }

    fn begin_region_select(&mut self, hwnd: isize) {
        let Some(overlay) = self.overlay.as_ref() else {
            self.state.write().set_error("overlay is not available".to_string());
            return;
        };
        if let Err(e) = overlay.attach(hwnd) {
            self.state.write().set_error(format!("region select: {e}"));
            return;
        }
        let regions = self.state.read().ocr_regions.clone();
        if let Err(e) = overlay.begin_region_select(regions.clone()) {
            self.state.write().set_error(format!("region select: {e}"));
            return;
        }
        let mut s = self.state.write();
        if s.target_hwnd.is_none() {
            s.target_hwnd = Some(hwnd);
        }
        s.region_select_active = true;
        s.region_select_draft = regions;
        info!("region picker started");
    }

    fn apply_overlay_event(&mut self, ev: OverlayEvent) {
        match ev {
            OverlayEvent::RegionsCommitted(regions) => {
                info!(count = regions.len(), "OCR regions committed");
                self.apply_ocr_regions(regions);
            }
            OverlayEvent::RegionSelectCancelled => {
                let mut s = self.state.write();
                s.region_select_active = false;
                s.region_select_draft.clear();
                info!("region picker cancelled");
            }
            OverlayEvent::RegionSelectUpdated(regions) => {
                let mut s = self.state.write();
                s.region_select_draft = regions;
            }
        }
    }

    fn stop_capture(&mut self) {
        if let Some(o) = self.overlay.as_ref() {
            let _ = o.cancel_region_select();
        }
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

    /// Reset gate / timers / captions. Optionally clear LLM conversation.
    fn reset_ocr_session(&mut self, clear_conversation: bool) {
        self.gate.reset_all();
        self.persist.reset();
        if clear_conversation {
            self.conversation.clear();
            self.client.reset_session();
        }
        self.last_translated_fp = None;
        self.last_page = None;
        self.raw_empty_since = None;
        self.raw_content_since = None;
        self.remap_miss_since = None;
        // Drop OCR + caption state so sticky remap cannot resurrect a prior session.
        let mut s = self.state.write();
        s.latest_ocr_blocks.clear();
        s.latest_ocr_text.clear();
        s.latest_translated_blocks.clear();
        s.latest_translated_text.clear();
        s.can_retry_translate = false;
        s.translate_in_flight = false;
    }

    async fn manual_capture(&mut self) {
        if self.inflight.is_some() {
            warn!("manual capture ignored — translation in flight (cancel first)");
            return;
        }
        if !self.ensure_engine().await {
            return;
        }

        self.session.sync_stream();
        match self.session.latest_frame() {
            Some(frame) => {
                self.update_preview(&frame);
                self.run_ocr_manual(&frame).await;
            }
            None => self.state.write().set_error("no capture frame"),
        }
    }

    async fn drive_capture(&mut self) {
        if !self.session.is_running() || self.inflight.is_some() {
            return;
        }

        self.session.sync_stream();
        if let Some(frame) = self.session.latest_frame() {
            self.update_preview(&frame);
            if self.engine.is_none() {
                let mut s = self.state.write();
                if !matches!(
                    s.status,
                    PipelineStatus::Error { .. }
                        | PipelineStatus::LoadingModels
                        | PipelineStatus::Translating
                        | PipelineStatus::RetryingTranslate { .. }
                        | PipelineStatus::OverlayActive
                        | PipelineStatus::Cancelled
                ) {
                    s.status = PipelineStatus::Capturing;
                }
            } else {
                self.run_ocr_auto(&frame).await;
            }
        }

        self.expire_pending();
    }
}

enum PipelineEvent {
    Command(Option<PipelineCommand>),
    Translate(Result<TranslateJobResult, oneshot::error::RecvError>),
    Overlay(Option<OverlayEvent>),
    Tick,
}

async fn next_event(
    rx: &mut CmdRx,
    inflight: Option<&mut InflightTranslate>,
    overlay: Option<&mut OverlayController>,
    wait: Option<Duration>,
) -> PipelineEvent {
    let overlay_event = async {
        match overlay {
            Some(o) => o.recv_event().await,
            None => std::future::pending().await,
        }
    };

    if let Some(job) = inflight {
        tokio::select! {
            biased;
            cmd = rx.recv() => PipelineEvent::Command(cmd),
            result = &mut job.rx => PipelineEvent::Translate(result),
            ev = overlay_event => PipelineEvent::Overlay(ev),
            () = sleep_or_pending(wait) => PipelineEvent::Tick,
        }
    } else {
        tokio::select! {
            biased;
            cmd = rx.recv() => PipelineEvent::Command(cmd),
            ev = overlay_event => PipelineEvent::Overlay(ev),
            () = sleep_or_pending(wait) => PipelineEvent::Tick,
        }
    }
}

async fn sleep_or_pending(wait: Option<Duration>) {
    match wait {
        Some(d) => tokio::time::sleep(d).await,
        None => std::future::pending().await,
    }
}

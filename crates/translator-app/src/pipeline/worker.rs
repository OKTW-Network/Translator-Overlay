//! Pipeline worker: owns capture / OCR / translate state and the command loop.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use parking_lot::RwLock;
use tokio::sync::{mpsc, oneshot, watch};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use translator_capture::{CaptureSession, CapturedFrame};
use translator_core::{AppState, ModelTier, OcrBlock, PipelineStatus};
use translator_ocr::{BlockPersistenceFilter, ModelLoadUpdate, OcrEngine, OcrFingerprint, StabilityGate};
use translator_overlay::{OverlayController, OverlayEvent};
use translator_translate::{Completion, Conversation, TranslateClient, TranslateError, TranslationCache};

use crate::pipeline::{PipelineCommand, ping_ui};

pub type SharedState = Arc<RwLock<AppState>>;
pub type CmdRx = mpsc::UnboundedReceiver<PipelineCommand>;
pub type CmdTx = mpsc::UnboundedSender<PipelineCommand>;

pub(crate) struct InflightTranslate {
    pub cancel: CancellationToken,
    pub rx: oneshot::Receiver<Result<Completion, TranslateError>>,
    pub fingerprint: OcrFingerprint,
    pub blocks: Vec<OcrBlock>,
    pub source_text: String,
    pub content_width: u32,
    pub content_height: u32,
    /// Unique miss blocks actually sent to the model (ids preserved).
    pub miss_blocks: Vec<OcrBlock>,
    /// Per-source-block cache hits (`None` = wait for the model / fallback).
    pub cached_hits: Vec<Option<String>>,
}

/// Background OCR model download + ORT session build (`watch` = latest phase only).
pub(crate) struct InflightModelLoad {
    pub rx: watch::Receiver<ModelLoadUpdate>,
    pub tier: ModelTier,
}

#[derive(Clone)]
pub(crate) struct PendingPage {
    pub blocks: Vec<OcrBlock>,
    pub source_text: String,
    pub fingerprint: OcrFingerprint,
    pub content_width: u32,
    pub content_height: u32,
}

/// Raw OCR of one capture frame, keyed by frame identity.
///
/// Static windows stop producing WGC frames; re-running inference on the same
/// frame would burn GPU for identical output. Only the inference is skipped —
/// persist / stability gate / remap still consume the cached blocks every tick,
/// which is how the gate accumulates its stable-duration clock.
pub(crate) struct LastRawOcr {
    pub sequence: u64,
    pub width: u32,
    pub height: u32,
    pub blocks: Vec<OcrBlock>,
}

impl LastRawOcr {
    pub(crate) fn matches(&self, frame: &CapturedFrame) -> bool {
        self.sequence == frame.sequence && self.width == frame.width && self.height == frame.height
    }
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
    pub model_load: Option<InflightModelLoad>,
    pub ocr_tier: ModelTier,
    pub last_raw_ocr: Option<LastRawOcr>,
    /// Wall-clock: raw OCR first went empty (may still have hysteresis tracks).
    pub raw_empty_since: Option<Instant>,
    /// Wall-clock since raw OCR last became non-empty.
    pub raw_content_since: Option<Instant>,
    /// Sticky remap failed while captions still present.
    pub remap_miss_since: Option<Instant>,
    pub overlay: Option<OverlayController>,
}

pub fn spawn_pipeline(state: SharedState) -> (CmdTx, tokio::task::JoinHandle<()>) {
    let (cmd_tx, rx) = mpsc::unbounded_channel();
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
            conversation: Conversation::empty(),
            translation_cache: TranslationCache::new(cache_max),
            client,
            last_translated_fp: None,
            last_page: None,
            inflight: None,
            model_load: None,
            ocr_tier,
            last_raw_ocr: None,
            raw_empty_since: None,
            raw_content_since: None,
            remap_miss_since: None,
            overlay,
        };

        // Kick off download + load without blocking the command loop / UI.
        pipeline.start_model_load();
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
            ping_ui();
            let wait = self.next_wait();
            match next_event(&mut rx, self.inflight.as_mut(), self.model_load.as_mut(), self.overlay.as_mut(), wait).await {
                PipelineEvent::Command(None) => return,
                PipelineEvent::Command(Some(cmd)) => {
                    if self.handle_command(cmd).await {
                        return;
                    }
                }
                PipelineEvent::Translate(result) => {
                    let job = self.inflight.take().expect("inflight present");
                    let result = result.unwrap_or_else(|_| Err(TranslateError::Other("translate task dropped".into())));
                    self.finish_translate(job, result);
                }
                PipelineEvent::ModelLoad => self.sync_model_load(),
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
                PipelineEvent::Tick => continue,
            }
            ping_ui();
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
                self.model_load = None;
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
            PipelineCommand::ApplyConfig(cfg) => self.apply_config(*cfg).await,
            PipelineCommand::SetOverlayDisplay { enabled, reader_enabled } => self.set_overlay_display(enabled, reader_enabled),
            PipelineCommand::StopCapture => self.stop_capture(),
            PipelineCommand::BeginRegionSelect { hwnd } => self.begin_region_select(hwnd),
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
                if self.model_load.is_some() {
                    warn!("start capture ignored — OCR models still loading");
                    self.state.write().last_error = Some("OCR models are still downloading or loading".into());
                } else if self.engine.is_none() {
                    self.state.write().set_error("OCR engine is not ready".to_string());
                } else {
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
            }
            PipelineCommand::ManualCapture => self.manual_capture().await,
            PipelineCommand::ResetConversation => {
                self.conversation.clear();
                self.client.reset_session();
                self.last_translated_fp = None;
                info!("LLM conversation reset");
                // Leave Downloading/Loading alone while models are still loading.
                if self.model_load.is_none() {
                    self.state.write().restore_operational_status();
                }
            }
            PipelineCommand::ClearTranslationCache => {
                self.translation_cache.clear();
                self.state.write().translation_cache_len = 0;
                info!("translation cache cleared");
                if self.model_load.is_none() {
                    self.state.write().restore_operational_status();
                }
            }
        }
        false
    }

    pub(crate) fn cancel_inflight(&mut self) {
        if let Some(job) = self.inflight.take() {
            job.cancel.cancel();
            // Drop the pending user turn so stop/shutdown mid-request does not
            // leave a dangling multi-user conversation for the next translate.
            self.conversation.rollback_user_turn();
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
        self.last_raw_ocr = None;
        self.raw_empty_since = None;
        self.raw_content_since = None;
        self.remap_miss_since = None;
        // Drop OCR + caption state so sticky remap cannot resurrect a prior session.
        let mut s = self.state.write();
        s.latest_ocr_blocks.clear();
        s.latest_translated_blocks.clear();
        s.translate_in_flight = false;
    }

    pub(crate) fn update_preview(&self, frame: &CapturedFrame) {
        let mut s = self.state.write();
        // Skip the write when the frame identity is unchanged (stale tick).
        if s.preview.sequence == frame.sequence && s.preview.width == frame.width && s.preview.height == frame.height {
            return;
        }
        s.preview.width = frame.width;
        s.preview.height = frame.height;
        s.preview.sequence = frame.sequence;
        s.preview.rgba = Some(frame.rgba.clone());
    }

    async fn manual_capture(&mut self) {
        if self.inflight.is_some() {
            warn!("manual capture ignored — translation in flight (cancel first)");
            return;
        }
        if self.model_load.is_some() {
            warn!("manual capture ignored — OCR models still loading");
            self.state.write().last_error = Some("OCR models are still downloading or loading".into());
            return;
        }
        if !self.ensure_engine() {
            return;
        }

        if self.session.in_movesize() {
            warn!("manual capture ignored — target is being moved or resized");
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
        // Skip OCR / preview / caption expiry while the target is in its
        // move/size loop. Frames are not published until MOVESIZEEND.
        if self.session.in_movesize() {
            return;
        }

        // A restarted stream renumbers frames from 0 — drop the raw-OCR cache so a
        // repeated sequence is not mistaken for an unchanged frame.
        if self.session.sync_stream() {
            self.last_raw_ocr = None;
        }
        if let Some(frame) = self.session.latest_frame() {
            self.update_preview(&frame);
            if self.engine.is_none() {
                // Do not auto-retry after a failed load — wait for ApplyConfig / tier change.
                let load_failed = matches!(self.state.read().status, PipelineStatus::Error { .. });
                if !load_failed {
                    let _ = self.ensure_engine();
                }
                let mut s = self.state.write();
                if !matches!(
                    s.status,
                    PipelineStatus::Error { .. }
                        | PipelineStatus::DownloadingModels { .. }
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
    Translate(Result<Result<Completion, TranslateError>, oneshot::error::RecvError>),
    ModelLoad,
    Overlay(Option<OverlayEvent>),
    Tick,
}

async fn next_event(
    rx: &mut CmdRx,
    inflight: Option<&mut InflightTranslate>,
    model_load: Option<&mut InflightModelLoad>,
    overlay: Option<&mut OverlayController>,
    wait: Option<Duration>,
) -> PipelineEvent {
    let overlay_event = async {
        match overlay {
            Some(o) => o.recv_event().await,
            None => std::future::pending().await,
        }
    };

    let translate = async {
        match inflight {
            Some(job) => (&mut job.rx).await,
            None => std::future::pending().await,
        }
    };
    let model = async {
        match model_load {
            Some(m) => {
                let _ = m.rx.changed().await;
            }
            None => std::future::pending().await,
        }
    };

    tokio::select! {
        biased;
        cmd = rx.recv() => PipelineEvent::Command(cmd),
        result = translate => PipelineEvent::Translate(result),
        _ = model => PipelineEvent::ModelLoad,
        ev = overlay_event => PipelineEvent::Overlay(ev),
        () = sleep_or_pending(wait) => PipelineEvent::Tick,
    }
}

async fn sleep_or_pending(wait: Option<Duration>) {
    match wait {
        Some(d) => tokio::time::sleep(d).await,
        None => std::future::pending().await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(w: u32, h: u32, seq: u64) -> CapturedFrame {
        CapturedFrame::new(w, h, vec![0u8; (w * h * 4) as usize], seq)
    }

    #[test]
    fn last_raw_ocr_matches_only_same_frame_identity() {
        let cache = LastRawOcr {
            sequence: 7,
            width: 320,
            height: 240,
            blocks: Vec::new(),
        };
        assert!(cache.matches(&frame(320, 240, 7)));
        assert!(!cache.matches(&frame(320, 240, 8)), "new sequence must miss");
        assert!(!cache.matches(&frame(640, 480, 7)), "new dimensions must miss");
    }
}

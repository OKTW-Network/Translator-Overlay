//! Pipeline worker: owns capture / OCR / translate state and the command loop.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use bytes::Bytes;
use parking_lot::RwLock;
use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use translator_capture::{CaptureSession, CapturedFrame};
use translator_core::{AppState, ModelTier, OcrBlock, PipelineStatus, Rect, TranslatedBlock, normalize_ocr_text};
use translator_ocr::{BlockPersistenceFilter, ModelLoadUpdate, OcrEngine, OcrFingerprint, StabilityGate};
use translator_overlay::{OverlayCommand, OverlayController, OverlayEvent};
use translator_translate::{Completion, Conversation, TranslateClient, TranslateError, TranslationCache};

use crate::pipeline::{PipelineCommand, ping_ui};

pub type SharedState = Arc<RwLock<AppState>>;
pub type CmdRx = mpsc::UnboundedReceiver<PipelineCommand>;
pub type CmdTx = mpsc::UnboundedSender<PipelineCommand>;

pub(crate) enum TranslateJobMsg {
    Partial(Vec<(u32, String)>),
    Done(Result<Completion, TranslateError>),
}

pub(crate) struct InflightTranslate {
    pub cancel: CancellationToken,
    pub rx: mpsc::UnboundedReceiver<TranslateJobMsg>,
    pub fingerprint: OcrFingerprint,
    pub blocks: Vec<OcrBlock>,
    pub content_width: u32,
    pub content_height: u32,
    /// Unique miss blocks actually sent to the model (ids preserved).
    pub miss_blocks: Vec<OcrBlock>,
    /// Per-source-block cache hits (`None` = wait for the model / fallback).
    pub cached_hits: Vec<Option<String>>,
    /// Overlay at job start (cache-hit preview or last committed page). Restored on retry / fail.
    pub revert_blocks: Vec<TranslatedBlock>,
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
/// which is how the gate accumulates its stable-duration clock. Continuously
/// redrawing windows (games) get new sequences; [`LastRawOcr::same_content`]
/// catches repaints whose OCR scope is pixel-identical.
pub(crate) struct LastRawOcr {
    pub sequence: u64,
    pub width: u32,
    pub height: u32,
    /// Pixels the OCR result depends on: one packed RGBA crop per OCR rect
    /// (whole frame when no regions are configured). Pixels outside the rects
    /// never affect recognition and are neither stored nor compared.
    pub scope: Vec<Bytes>,
    pub blocks: Vec<OcrBlock>,
}

impl LastRawOcr {
    /// Pack the OCR scope of `frame`: each rect's rows, or the whole frame when
    /// no regions are configured.
    pub(crate) fn pack_scope(frame: &CapturedFrame, rects: &[Rect]) -> Vec<Bytes> {
        if rects.is_empty() {
            return vec![frame.rgba.clone()];
        }
        rects.iter().map(|r| pack_rect(frame, *r)).collect()
    }

    /// Same scope pixels — an OCR pass would return the same blocks.
    ///
    /// Compares RGBA, a superset of what OCR reads (alpha is dropped): an
    /// alpha-only change costs one redundant inference, never a stale hit.
    pub(crate) fn same_content(&self, frame: &CapturedFrame, rects: &[Rect]) -> bool {
        if self.width != frame.width || self.height != frame.height {
            return false;
        }
        // Malformed buffer — miss instead of slicing out of bounds.
        if frame.rgba.len() != frame.width as usize * frame.height as usize * 4 {
            return false;
        }
        if rects.is_empty() {
            // Whole-window scope: one packed buffer covering the full frame.
            return self.scope.len() == 1 && self.scope[0] == frame.rgba;
        }
        self.scope.len() == rects.len() && self.scope.iter().zip(rects).all(|(stored, r)| rect_unchanged(frame, *r, stored))
    }
}

/// Pack one rect's rows out of a tightly packed RGBA frame.
fn pack_rect(frame: &CapturedFrame, rect: Rect) -> Bytes {
    let (x0, y0, x1, y1) = rect.clamped_bounds(frame.width, frame.height);
    let cw = (x1 - x0) as usize;
    let row_w = frame.width as usize;
    let mut out = Vec::with_capacity(cw * (y1 - y0) as usize * 4);
    for y in y0..y1 {
        let start = (y as usize * row_w + x0 as usize) * 4;
        out.extend_from_slice(&frame.rgba[start..start + cw * 4]);
    }
    Bytes::from(out)
}

/// Row-wise memcmp of one rect against its packed copy (early-exit on first row diff).
fn rect_unchanged(frame: &CapturedFrame, rect: Rect, stored: &Bytes) -> bool {
    let (x0, y0, x1, y1) = rect.clamped_bounds(frame.width, frame.height);
    let cw = (x1 - x0) as usize;
    if stored.len() != cw * (y1 - y0) as usize * 4 {
        return false;
    }
    let row_w = frame.width as usize;
    for (row, y) in (y0..y1).enumerate() {
        let start = (y as usize * row_w + x0 as usize) * 4;
        if stored[row * cw * 4..(row + 1) * cw * 4] != frame.rgba[start..start + cw * 4] {
            return false;
        }
    }
    true
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
    /// Raise-on-restart offset added to frame sequences in preview state, so a
    /// renumbered stream never reuses a sequence the preview skip / UI cache saw.
    pub preview_seq_bias: u64,
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
            preview_seq_bias: 0,
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
                PipelineEvent::Translate(msg) => match msg {
                    Some(TranslateJobMsg::Partial(pairs)) => {
                        if let Some(job) = self.inflight.as_ref() {
                            self.apply_stream_preview(job, &pairs);
                        }
                    }
                    Some(TranslateJobMsg::Done(result)) => {
                        let job = self.inflight.take().expect("inflight present");
                        self.finish_translate(job, result);
                    }
                    None => {
                        let job = self.inflight.take().expect("inflight present");
                        self.finish_translate(job, Err(TranslateError::Other("translate task dropped".into())));
                    }
                },
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
                    let _ = o.send(OverlayCommand::Clear);
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
                    let _ = o.send(OverlayCommand::ConfirmRegionSelect);
                }
            }
            PipelineCommand::ClearRegionSelect => {
                if let Some(o) = self.overlay.as_ref() {
                    let _ = o.send(OverlayCommand::ClearRegionSelect);
                }
                let mut s = self.state.write();
                s.region_select_draft.clear();
            }
            PipelineCommand::SetCaptureRegions { regions } => {
                if let Some(o) = self.overlay.as_ref() {
                    let _ = o.send(OverlayCommand::CancelRegionSelect);
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
                self.state.write().history.clear();
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
        // The fresh stream renumbers frames from 1.
        self.absorb_preview_sequence();
        if let Some(o) = self.overlay.as_ref() {
            let _ = o.send(OverlayCommand::Clear);
            if let Some(hwnd) = self.session.target_hwnd() {
                let _ = o.send(OverlayCommand::Attach { target_hwnd: hwnd });
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
        if let Err(e) = overlay.send(OverlayCommand::Attach { target_hwnd: hwnd }) {
            self.state.write().set_error(format!("region select: {e}"));
            return;
        }
        let regions = self.state.read().ocr_regions.clone();
        if let Err(e) = overlay.send(OverlayCommand::BeginRegionSelect { regions: regions.clone() }) {
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
            let _ = o.send(OverlayCommand::CancelRegionSelect);
        }
        self.cancel_inflight();
        self.session.stop();
        self.reset_ocr_session(false);
        if let Some(o) = self.overlay.as_ref() {
            let _ = o.send(OverlayCommand::Detach);
            let _ = o.send(OverlayCommand::Clear);
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
        let sequence = frame.sequence.saturating_add(self.preview_seq_bias);
        let mut s = self.state.write();
        if s.preview.sequence == sequence && s.preview.width == frame.width && s.preview.height == frame.height {
            return;
        }
        s.preview.width = frame.width;
        s.preview.height = frame.height;
        s.preview.sequence = sequence;
        s.preview.rgba = Some(frame.rgba.clone());
    }

    /// A restarted stream renumbers frames from 1 — drop the raw-OCR cache and
    /// raise the preview sequence bias so a repeated sequence is never mistaken
    /// for an unchanged frame.
    fn consume_stream_restart(&mut self) {
        if self.session.sync_stream() {
            self.last_raw_ocr = None;
            self.absorb_preview_sequence();
        }
    }

    /// Absorb the last published preview sequence into the bias: a renumbered
    /// stream's frames then publish strictly higher sequences, so the preview
    /// skip and the UI preview cache never match a stale entry.
    fn absorb_preview_sequence(&mut self) {
        let last = self.state.read().preview.sequence;
        self.preview_seq_bias = self.preview_seq_bias.saturating_add(last);
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
        self.consume_stream_restart();
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

        self.consume_stream_restart();

        // Stale tick (no new frame published): consume the cached blocks without
        // the client-area crop, Win32 queries, or a preview write.
        let stale = self
            .last_raw_ocr
            .as_ref()
            .filter(|c| Some(c.sequence) == self.session.latest_sequence())
            .map(|c| (c.sequence, c.blocks.clone(), c.width, c.height));

        if let Some((sequence, blocks, w, h)) = stale {
            debug!(frame = sequence, "capture frame unchanged — reusing cached OCR");
            self.consume_raw_ocr(blocks, w, h).await;
        } else if let Some(frame) = self.session.latest_frame() {
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
    Translate(Option<TranslateJobMsg>),
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
            Some(job) => job.rx.recv().await,
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

/// Rebuild displayed translations from current OCR.
///
/// A caption is valid only while its normalized source string is still in OCR.
/// Different text is **not** reused — a new box appears only after
/// `finish_translate`. Same-text remaps keep the previous box (no follow-move)
/// so persist/detector swings cannot walk or re-fit the caption. Capture-surface
/// resize takes the latest OCR box so scale stays correct.
pub(crate) fn remap_translations_to_ocr(translated: &[TranslatedBlock], ocr: &[OcrBlock], content_resized: bool) -> Vec<TranslatedBlock> {
    let mut used = vec![false; translated.len()];
    let mut out = Vec::new();

    for (i, ob) in ocr.iter().enumerate() {
        let key = normalize_ocr_text(&ob.text);
        if key.is_empty() {
            continue;
        }

        let text_hit = translated.iter().enumerate().find_map(|(ti, t)| {
            if used[ti] {
                return None;
            }
            if normalize_ocr_text(&t.source) == key { Some(ti) } else { None }
        });

        let Some(ti) = text_hit else {
            continue;
        };
        used[ti] = true;
        let tb = &translated[ti];
        let bbox = if content_resized { ob.bbox } else { tb.bbox };
        out.push(TranslatedBlock {
            id: i as u32,
            source: tb.source.clone(),
            translation: tb.translation.clone(),
            confidence: ob.confidence,
            bbox,
            source_lines: tb.source_lines.max(1),
        });
    }
    out
}

/// True when the remapped overlay set differs in count, text, or bbox.
///
/// Match by source+translation (not zip order): OCR reorder must not force a
/// repaint that re-runs label layout and looks like the captions moved.
pub(crate) fn translated_geometry_changed(previous: &[TranslatedBlock], remapped: &[TranslatedBlock]) -> bool {
    if previous.len() != remapped.len() {
        return true;
    }
    for b in remapped {
        let key = normalize_ocr_text(&b.source);
        let Some(a) = previous
            .iter()
            .find(|p| normalize_ocr_text(&p.source) == key && p.translation == b.translation)
        else {
            return true;
        };
        if a.bbox != b.bbox {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(w: u32, h: u32, seq: u64) -> CapturedFrame {
        CapturedFrame::new(w, h, vec![0u8; (w * h * 4) as usize], seq)
    }

    fn cache(w: u32, h: u32, seq: u64, px: u8) -> LastRawOcr {
        LastRawOcr {
            sequence: seq,
            width: w,
            height: h,
            scope: vec![Bytes::from(vec![px; (w * h * 4) as usize])],
            blocks: Vec::new(),
        }
    }

    #[test]
    fn same_content_whole_frame_compares_all_pixels() {
        let cache = cache(8, 8, 1, 5);
        let same_pixels = CapturedFrame::new(8, 8, vec![5u8; 8 * 8 * 4], 99);
        assert!(cache.same_content(&same_pixels, &[]), "identical pixels must hit");

        let mut other = same_pixels.clone();
        other.rgba = Bytes::from(vec![6u8; 8 * 8 * 4]);
        assert!(!cache.same_content(&other, &[]), "different pixels must miss");

        assert!(!cache.same_content(&frame(16, 4, 1), &[]), "different dimensions must miss");
    }

    #[test]
    fn same_content_short_buffer_is_miss() {
        let cache = cache(8, 8, 1, 5);
        let short = CapturedFrame::new(8, 8, vec![5u8; 10], 2);
        assert!(!cache.same_content(&short, &[]), "short whole-frame buffer must miss");
        assert!(!cache.same_content(&short, &[Rect::new(0.0, 0.0, 4.0, 4.0)]), "short buffer must miss before any rect slicing");
    }

    #[test]
    fn same_content_ignores_pixels_outside_regions() {
        // 32×32 frame; scope = one 8×8 region at (4, 4).
        let rect = Rect::new(4.0, 4.0, 8.0, 8.0);
        let mut base = vec![7u8; 32 * 32 * 4];
        let src = CapturedFrame::new(32, 32, base.clone(), 1);
        let cache = LastRawOcr {
            sequence: 1,
            width: 32,
            height: 32,
            scope: LastRawOcr::pack_scope(&src, &[rect]),
            blocks: Vec::new(),
        };

        // Change a pixel OUTSIDE the region (31, 31): still a content hit.
        let i = (31 * 32 + 31) as usize * 4;
        base[i] = 9;
        let outside = CapturedFrame::new(32, 32, base, 2);
        assert!(cache.same_content(&outside, &[rect]), "changes outside regions must hit");

        // Change a pixel INSIDE the region (5, 5): miss.
        let mut inside = cache.scope[0].to_vec();
        // crop-local (1, 1) in an 8-wide crop → (8 + 1) px, 4 bytes/px
        inside[(8 + 1) * 4] = 9;
        let scope = vec![Bytes::from(inside)];
        let cache = LastRawOcr { scope, ..cache };
        assert!(!cache.same_content(&src, &[rect]), "changes inside regions must miss");

        // Different region count: miss.
        assert!(!cache.same_content(&src, &[rect, rect]));
    }

    fn remap_ocr(text: &str, bbox: Rect) -> OcrBlock {
        OcrBlock {
            id: 0,
            text: text.to_string(),
            confidence: 0.9,
            bbox,
            source_lines: 1,
        }
    }

    fn remap_translated(source: &str, translation: &str, bbox: Rect) -> TranslatedBlock {
        TranslatedBlock {
            id: 0,
            source: source.to_string(),
            translation: translation.to_string(),
            confidence: 0.9,
            bbox,
            source_lines: 1,
        }
    }

    #[test]
    fn remap_keeps_size_when_same_source_grows() {
        let prev_box = Rect::new(100.0, 200.0, 80.0, 24.0);
        let wider = Rect::new(100.0, 200.0, 160.0, 24.0);
        let prior = [remap_translated("セリフ", "line", prev_box)];
        let now = [remap_ocr("セリフ", wider)];
        let out = remap_translations_to_ocr(&prior, &now, false);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].translation, "line");
        assert_eq!(out[0].bbox, prev_box);
    }

    #[test]
    fn remap_freezes_box_until_retranslate() {
        let prev_box = Rect::new(100.0, 200.0, 180.0, 28.0);
        let moved = Rect::new(100.0, 320.0, 180.0, 28.0);
        let prior = [remap_translated("menu", "選單", prev_box)];
        let now = [remap_ocr("menu", moved)];
        let out = remap_translations_to_ocr(&prior, &now, false);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].bbox, prev_box, "same source must not walk the caption");
    }

    #[test]
    fn remap_drops_caption_when_source_differs() {
        let box_a = Rect::new(100.0, 200.0, 80.0, 24.0);
        let box_b = Rect::new(98.0, 198.0, 160.0, 26.0);
        let prior = [remap_translated("セリフ", "old line", box_a)];
        let now = [remap_ocr("次の台詞", box_b)];
        let out = remap_translations_to_ocr(&prior, &now, false);
        assert!(out.is_empty(), "different source must not reuse the caption");
    }

    #[test]
    fn remap_hits_across_ellipsis_length() {
        let prev_box = Rect::new(100.0, 200.0, 80.0, 22.0);
        let wider = Rect::new(96.0, 198.0, 170.0, 26.0);
        let prior = [remap_translated("待って…", "等等", prev_box)];
        let now = [remap_ocr("待って………", wider)];
        let out = remap_translations_to_ocr(&prior, &now, false);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].translation, "等等");
        assert_eq!(out[0].bbox, prev_box);
        assert!(!translated_geometry_changed(&prior, &out));
    }

    #[test]
    fn remap_hits_across_fullwidth_question() {
        let prev_box = Rect::new(100.0, 200.0, 80.0, 22.0);
        let jitter = Rect::new(102.0, 201.0, 76.0, 20.0);
        let prior = [remap_translated("何？", "什麼？", prev_box)];
        let now = [remap_ocr("何?", jitter)];
        let out = remap_translations_to_ocr(&prior, &now, false);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].bbox, prev_box);
        assert!(!translated_geometry_changed(&prior, &out));
    }

    #[test]
    fn remap_adopts_ocr_box_after_content_resize() {
        let prev_box = Rect::new(50.0, 80.0, 100.0, 20.0);
        let scaled = Rect::new(75.0, 120.0, 150.0, 30.0);
        let prior = [remap_translated("hello", "你好", prev_box)];
        let now = [remap_ocr("hello", scaled)];
        let out = remap_translations_to_ocr(&prior, &now, true);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].bbox, scaled);
    }
}

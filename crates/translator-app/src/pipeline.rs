//! Background pipeline: capture → OCR → stability gate → LLM translate → overlay.

use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use parking_lot::RwLock;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use translator_capture::{CaptureSession, CapturedFrame};
use translator_core::{AppConfig, AppState, OcrBlock, PipelineStatus, TranslatedBlock};
use translator_ocr::{BlockPersistenceFilter, OcrEngine, OcrFingerprint, StabilityGate, StabilityOutcome, prepare_engine};
use translator_overlay::OverlayController;
use translator_translate::{Conversation, TranslateClient, TranslateError, blocks_to_translated_text, merge_translations};

/// Commands the UI sends to the pipeline worker.
#[derive(Debug)]
pub enum PipelineCommand {
    StartCapture {
        hwnd: isize,
        title: String,
    },
    StartForeground,
    StopCapture,
    /// Grab one frame and OCR + translate immediately (bypass stability wait).
    ManualCapture,
    SetShowPreview(bool),
    /// Drop LLM conversation history (keeps OCR models).
    ResetConversation,
    /// Apply a full config snapshot (saved from the settings UI).
    ApplyConfig(Box<AppConfig>),
    /// Cancel the in-flight translation request (if any).
    CancelTranslate,
    /// Re-run translation on the latest OCR blocks.
    RetryTranslate,
    Shutdown,
}

pub type SharedState = Arc<RwLock<AppState>>;
pub type CmdTx = std::sync::mpsc::Sender<PipelineCommand>;
pub type CmdRx = std::sync::mpsc::Receiver<PipelineCommand>;

/// Result of a background translate HTTP job.
struct TranslateJobResult {
    result: Result<String, TranslateError>,
    /// Conversation messages length after pushing the user turn (for rollback).
    messages_len_after_user: usize,
}

struct InflightTranslate {
    cancel: CancellationToken,
    rx: oneshot::Receiver<TranslateJobResult>,
    fingerprint: OcrFingerprint,
    blocks: Vec<OcrBlock>,
    source_text: String,
    content_width: u32,
    content_height: u32,
}

#[derive(Clone)]
struct PendingPage {
    blocks: Vec<OcrBlock>,
    source_text: String,
    fingerprint: OcrFingerprint,
    content_width: u32,
    content_height: u32,
}

/// Mutable pipeline pieces shared by OCR → translate helpers.
struct TranslateCtx<'a> {
    state: &'a SharedState,
    rt: &'a tokio::runtime::Runtime,
    client: &'a mut TranslateClient,
    conversation: &'a mut Conversation,
    inflight: &'a mut Option<InflightTranslate>,
    last_page: &'a mut Option<PendingPage>,
    last_translated_fp: &'a mut Option<OcrFingerprint>,
    overlay: Option<&'a OverlayController>,
}

/// OCR timing clocks shared across auto-capture frames.
struct OcrTimers<'a> {
    raw_empty_since: &'a mut Option<Instant>,
    raw_content_since: &'a mut Option<Instant>,
    remap_miss_since: &'a mut Option<Instant>,
}

/// Live gate + block persistence updated each OCR frame.
struct OcrFilters<'a> {
    gate: &'a mut StabilityGate,
    persist: &'a mut BlockPersistenceFilter,
}

/// Live OCR filters / engine state updated when config is applied.
struct ConfigApplyTargets<'a> {
    client: &'a mut TranslateClient,
    gate: &'a mut StabilityGate,
    persist: &'a mut BlockPersistenceFilter,
    show_preview: &'a mut bool,
    ocr_tier: &'a mut translator_core::ModelTier,
    engine: &'a mut Option<OcrEngine>,
    overlay: Option<&'a OverlayController>,
}

pub fn spawn_pipeline(state: SharedState, rx: CmdRx) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("pipeline".into())
        .spawn(move || pipeline_loop(state, rx))
        .expect("spawn pipeline thread")
}

fn pipeline_loop(state: SharedState, rx: CmdRx) {
    let mut session = CaptureSession::new();
    let mut show_preview = state.read().config.capture.show_preview;
    let ocr_cfg = state.read().config.ocr.clone();
    let mut gate = StabilityGate::from_config(&ocr_cfg);
    let mut persist = BlockPersistenceFilter::from_config(&ocr_cfg);
    let mut engine: Option<OcrEngine> = None;
    let mut conversation = Conversation::new();
    let mut client = TranslateClient::new(state.read().config.api.clone());
    let mut last_translated_fp: Option<OcrFingerprint> = None;
    let mut last_page: Option<PendingPage> = None;
    let mut inflight: Option<InflightTranslate> = None;
    let mut ocr_tier = state.read().config.ocr.model_tier;
    // Wall-clock: raw OCR first went empty (may still have hysteresis tracks).
    // Cleared when raw text returns. Used so static empty screens still expire
    // captions even if capture stops sending frames after the first blank frame.
    let mut raw_empty_since: Option<Instant> = None;
    // Wall-clock since raw OCR last became non-empty. Used to force-translate
    // when block persistence / fingerprints never settle.
    let mut raw_content_since: Option<Instant> = None;
    // Sticky remap failed (no text/spatial hit) while we still had captions.
    // Clear after grace so wrong sticky captions do not freeze forever.
    let mut remap_miss_since: Option<Instant> = None;
    // Graphics Capture often delivers only one frame for a fully static window.
    // Keep the last frame and re-OCR on the capture interval so block persistence
    // and the stability gate can advance on wall-clock without new WGC samples.
    let mut last_frame: Option<CapturedFrame> = None;
    let mut last_ocr_at: Option<Instant> = None;

    let mut overlay = match OverlayController::spawn(state.read().config.overlay.clone()) {
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

    // Load OCR engine on start (oar-ocr may auto-download missing models).
    {
        let cfg = state.read().config.ocr.clone();
        match load_engine(&state, &cfg) {
            Ok(e) => engine = Some(e),
            Err(e) => {
                error!(error = %e, "initial model load failed");
                state.write().set_error(format!("models: {e}"));
            }
        }
    }

    loop {
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                PipelineCommand::Shutdown => {
                    if let Some(job) = inflight.take() {
                        job.cancel.cancel();
                    }
                    session.stop();
                    if let Some(mut o) = overlay.take() {
                        let _ = o.clear();
                        o.shutdown();
                    }
                    info!("pipeline shutdown");
                    return;
                }
                PipelineCommand::CancelTranslate => {
                    if let Some(job) = inflight.as_ref() {
                        info!("cancelling in-flight translation");
                        job.cancel.cancel();
                    } else {
                        let mut s = state.write();
                        s.translate_in_flight = false;
                        if s.auto_running {
                            s.status = PipelineStatus::Capturing;
                        } else {
                            s.status = PipelineStatus::Cancelled;
                        }
                    }
                }
                PipelineCommand::RetryTranslate => {
                    if inflight.is_some() {
                        warn!("retry ignored — translation already in flight");
                        continue;
                    }
                    if let Some(page) = last_page.clone() {
                        // Force re-translate even if content fingerprint matches.
                        last_translated_fp = None;
                        start_translate(
                            &mut TranslateCtx {
                                state: &state,
                                rt: &rt,
                                client: &mut client,
                                conversation: &mut conversation,
                                inflight: &mut inflight,
                                last_page: &mut last_page,
                                last_translated_fp: &mut last_translated_fp,
                                overlay: overlay.as_ref(),
                            },
                            page,
                            true,
                        );
                    } else {
                        state.write().set_error("nothing to retry".to_string());
                    }
                }
                PipelineCommand::ApplyConfig(cfg) => {
                    apply_config(
                        &state,
                        ConfigApplyTargets {
                            client: &mut client,
                            gate: &mut gate,
                            persist: &mut persist,
                            show_preview: &mut show_preview,
                            ocr_tier: &mut ocr_tier,
                            engine: &mut engine,
                            overlay: overlay.as_ref(),
                        },
                        *cfg,
                    );
                }
                PipelineCommand::StopCapture => {
                    if let Some(job) = inflight.take() {
                        job.cancel.cancel();
                    }
                    session.stop();
                    gate.reset_all();
                    persist.reset();
                    last_translated_fp = None;
                    raw_empty_since = None;
                    raw_content_since = None;
                    remap_miss_since = None;
                    last_frame = None;
                    last_ocr_at = None;
                    if let Some(o) = overlay.as_ref() {
                        let _ = o.detach();
                        let _ = o.clear();
                    }
                    let mut s = state.write();
                    s.auto_running = false;
                    s.target_hwnd = None;
                    s.translate_in_flight = false;
                    s.status = PipelineStatus::Idle;
                }
                PipelineCommand::StartForeground => {
                    if let Some(job) = inflight.take() {
                        job.cancel.cancel();
                    }
                    let interval = state.read().config.capture.min_interval_ms;
                    match session.start_foreground(interval) {
                        Ok(()) => {
                            gate.reset_all();
                            persist.reset();
                            conversation.clear();
                            last_translated_fp = None;
                            raw_empty_since = None;
                            raw_content_since = None;
                            remap_miss_since = None;
                            last_frame = None;
                            last_ocr_at = None;
                            if let Some(o) = overlay.as_ref() {
                                let _ = o.clear();
                                if let Some(hwnd) = session.target_hwnd {
                                    let _ = o.attach(hwnd);
                                }
                            }
                            let mut s = state.write();
                            s.auto_running = true;
                            s.target_window_title = session.target_title.clone();
                            s.target_hwnd = session.target_hwnd;
                            s.translate_in_flight = false;
                            s.last_error = None;
                            s.status = PipelineStatus::Capturing;
                        }
                        Err(e) => {
                            error!(error = %e, "start foreground capture failed");
                            state.write().set_error(e.to_string());
                        }
                    }
                }
                PipelineCommand::StartCapture { hwnd, title } => {
                    if let Some(job) = inflight.take() {
                        job.cancel.cancel();
                    }
                    let interval = state.read().config.capture.min_interval_ms;
                    match session.start_window(hwnd, title.clone(), interval) {
                        Ok(()) => {
                            gate.reset_all();
                            persist.reset();
                            conversation.clear();
                            last_translated_fp = None;
                            raw_empty_since = None;
                            raw_content_since = None;
                            remap_miss_since = None;
                            last_frame = None;
                            last_ocr_at = None;
                            if let Some(o) = overlay.as_ref() {
                                let _ = o.clear();
                                let _ = o.attach(hwnd);
                            }
                            let mut s = state.write();
                            s.auto_running = true;
                            s.target_window_title = Some(title);
                            s.target_hwnd = Some(hwnd);
                            s.translate_in_flight = false;
                            s.last_error = None;
                            s.status = PipelineStatus::Capturing;
                        }
                        Err(e) => {
                            error!(error = %e, "start capture failed");
                            state.write().set_error(e.to_string());
                        }
                    }
                }
                PipelineCommand::ManualCapture => {
                    if inflight.is_some() {
                        warn!("manual capture ignored — translation in flight (cancel first)");
                        continue;
                    }
                    if engine.is_none() {
                        let cfg = state.read().config.ocr.clone();
                        match load_engine(&state, &cfg) {
                            Ok(e) => engine = Some(e),
                            Err(e) => {
                                state.write().set_error(format!("OCR: {e}"));
                                continue;
                            }
                        }
                    }

                    // One-shot: attach overlay to the window we are about to capture.
                    let one_shot = !session.is_running();
                    let frame = if one_shot {
                        let interval = state.read().config.capture.min_interval_ms;
                        if let Err(e) = session.start_foreground(interval) {
                            state.write().set_error(e.to_string());
                            continue;
                        }
                        if let Some(o) = overlay.as_ref()
                            && let Some(hwnd) = session.target_hwnd
                        {
                            let _ = o.attach(hwnd);
                        }
                        {
                            let mut s = state.write();
                            s.target_window_title = session.target_title.clone();
                            s.target_hwnd = session.target_hwnd;
                        }
                        let f = session.recv_frame_timeout(Duration::from_secs(2));
                        // Keep session hwnd for overlay; stop only the capture stream.
                        let hwnd = session.target_hwnd;
                        let title = session.target_title.clone();
                        session.stop();
                        // Restore target info for overlay after stop cleared it.
                        session.target_hwnd = hwnd;
                        session.target_title = title;
                        f
                    } else {
                        session
                            .take_latest_frame()
                            .or_else(|| session.recv_frame_timeout(Duration::from_millis(500)))
                    };

                    if let Some(frame) = frame {
                        update_preview(&state, &frame, show_preview);
                        if let Some(eng) = engine.as_mut() {
                            run_ocr_manual(
                                &mut TranslateCtx {
                                    state: &state,
                                    rt: &rt,
                                    client: &mut client,
                                    conversation: &mut conversation,
                                    inflight: &mut inflight,
                                    last_page: &mut last_page,
                                    last_translated_fp: &mut last_translated_fp,
                                    overlay: overlay.as_ref(),
                                },
                                eng,
                                &mut gate,
                                &frame,
                            );
                        }
                    } else {
                        state.write().set_error("manual capture timeout");
                    }
                }
                PipelineCommand::SetShowPreview(v) => {
                    show_preview = v;
                    state.write().config.capture.show_preview = v;
                    let _ = state.read().config.save_default_path();
                }
                PipelineCommand::ResetConversation => {
                    conversation.clear();
                    last_translated_fp = None;
                    info!("LLM conversation reset");
                    let mut s = state.write();
                    if s.translate_in_flight {
                        // keep Translating
                    } else if s.auto_running {
                        s.status = PipelineStatus::Capturing;
                    } else if !s.latest_translated_blocks.is_empty() {
                        s.status = PipelineStatus::OverlayActive;
                    } else {
                        s.status = PipelineStatus::Idle;
                    }
                }
            }
        }

        // Poll async translate completion without blocking the pipeline loop.
        if let Some(job) = inflight.as_mut() {
            match job.rx.try_recv() {
                Ok(job_result) => {
                    let finished = inflight.take().expect("inflight present");
                    finish_translate(&state, &mut conversation, overlay.as_ref(), finished, job_result, &mut last_translated_fp);
                }
                Err(oneshot::error::TryRecvError::Empty) => {}
                Err(oneshot::error::TryRecvError::Closed) => {
                    let finished = inflight.take().expect("inflight present");
                    let messages_len_after_user = conversation.messages.len();
                    finish_translate(
                        &state,
                        &mut conversation,
                        overlay.as_ref(),
                        finished,
                        TranslateJobResult {
                            result: Err(TranslateError::Other("translate task dropped".into())),
                            messages_len_after_user,
                        },
                        &mut last_translated_fp,
                    );
                }
            }
        }

        // Capture + OCR while not translating (OCR stays off the UI thread).
        if session.is_running() && inflight.is_none() {
            let interval_ms = state.read().config.capture.min_interval_ms.max(50);
            let stale_due = last_ocr_at
                .map(|t| t.elapsed() >= Duration::from_millis(interval_ms))
                .unwrap_or(true);

            // New WGC frames always OCR immediately. Fully static windows often
            // stop delivering frames entirely — re-OCR the last sample on the
            // capture interval so block persistence + stability still advance
            // and translation can fire without waiting for a screen change.
            let frame_for_ocr = match session.take_latest_frame() {
                Some(frame) => {
                    last_frame = Some(frame.clone());
                    Some(frame)
                }
                None if stale_due => last_frame.clone(),
                None => None,
            };

            if let Some(frame) = frame_for_ocr {
                last_ocr_at = Some(Instant::now());
                update_preview(&state, &frame, show_preview);

                if engine.is_none() {
                    let mut s = state.write();
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
                } else if let Some(eng) = engine.as_mut() {
                    run_ocr_auto(
                        &mut TranslateCtx {
                            state: &state,
                            rt: &rt,
                            client: &mut client,
                            conversation: &mut conversation,
                            inflight: &mut inflight,
                            last_page: &mut last_page,
                            last_translated_fp: &mut last_translated_fp,
                            overlay: overlay.as_ref(),
                        },
                        eng,
                        OcrFilters {
                            gate: &mut gate,
                            persist: &mut persist,
                        },
                        OcrTimers {
                            raw_empty_since: &mut raw_empty_since,
                            raw_content_since: &mut raw_content_since,
                            remap_miss_since: &mut remap_miss_since,
                        },
                        &frame,
                    );
                }
            } else {
                // Between OCR ticks: still expire captions after raw-empty grace
                // so blank screens do not keep the last translation forever.
                maybe_expire_raw_empty(
                    &mut TranslateCtx {
                        state: &state,
                        rt: &rt,
                        client: &mut client,
                        conversation: &mut conversation,
                        inflight: &mut inflight,
                        last_page: &mut last_page,
                        last_translated_fp: &mut last_translated_fp,
                        overlay: overlay.as_ref(),
                    },
                    &mut gate,
                    &mut persist,
                    &mut raw_empty_since,
                );
            }
        }

        std::thread::sleep(Duration::from_millis(50));
    }
}

fn apply_config(state: &SharedState, targets: ConfigApplyTargets<'_>, cfg: AppConfig) {
    // Model tier change requires a full engine reload (new ONNX weights).
    let engine_reload = *targets.ocr_tier != cfg.ocr.model_tier;
    *targets.show_preview = cfg.capture.show_preview;
    *targets.gate = StabilityGate::from_config(&cfg.ocr);
    *targets.persist = BlockPersistenceFilter::from_config(&cfg.ocr);
    targets.client.update_api(cfg.api.clone());

    if let Some(o) = targets.overlay {
        let _ = o.update_config(cfg.overlay.clone());
    }

    if let Err(e) = cfg.save_default_path() {
        error!(error = %e, "failed to save config");
        let mut s = state.write();
        s.config = cfg;
        s.set_error(format!("save config: {e}"));
        return;
    }

    {
        let mut s = state.write();
        s.config = cfg.clone();
        // Single short line for the UI InfoBar title (avoid title+message pair).
        s.settings_message = Some("Saved".into());
        s.last_error = None;
    }
    info!("config applied and saved");

    if engine_reload {
        // Reload weights / ORT session for the new tier.
        *targets.ocr_tier = cfg.ocr.model_tier;
        *targets.engine = None;
        match load_engine(state, &cfg.ocr) {
            Ok(e) => *targets.engine = Some(e),
            Err(e) => {
                state.write().set_error(format!("models: {e}"));
                return;
            }
        }
    } else if let Some(eng) = targets.engine.as_mut() {
        // Same engine: still pick up confidence / line-merge / filter knobs.
        eng.apply_runtime_config(&cfg.ocr);
    }

    // Restore a sensible non-error status after save.
    let mut s = state.write();
    if s.translate_in_flight {
        s.status = PipelineStatus::Translating;
    } else if s.auto_running {
        s.status = PipelineStatus::Capturing;
    } else if !s.latest_translated_blocks.is_empty() {
        s.status = PipelineStatus::OverlayActive;
    } else {
        s.status = PipelineStatus::Idle;
    }
}

fn load_engine(state: &SharedState, cfg: &translator_core::OcrConfig) -> Result<OcrEngine, translator_ocr::OcrError> {
    state.write().status = PipelineStatus::LoadingModels;
    // May block while oar-ocr fetches missing registry files / loads ORT.
    prepare_engine(cfg)
}

fn update_preview(state: &SharedState, frame: &CapturedFrame, show_preview: bool) {
    let mut s = state.write();
    s.frame_count = frame.sequence;
    s.preview.width = frame.width;
    s.preview.height = frame.height;
    s.preview.sequence = frame.sequence;

    if show_preview {
        match save_preview(frame) {
            Ok(path) => s.preview.path = Some(path.display().to_string()),
            Err(e) => warn!(error = %e, "failed to save preview"),
        }
    }
}

/// Drop stale captions so they do not stick on the overlay, and forget
/// fingerprints / persistence tracks so the same page can re-translate if it
/// reappears.
fn clear_stale_overlay(ctx: &mut TranslateCtx<'_>, gate: &mut StabilityGate, persist: &mut BlockPersistenceFilter, reason: &str) {
    let mut s = ctx.state.write();
    let had_content = !s.latest_translated_blocks.is_empty()
        || !s.latest_ocr_blocks.is_empty()
        || !s.latest_translated_text.is_empty()
        || !s.latest_ocr_text.is_empty();

    if !had_content {
        // Still warming up (icons/flicker filtered out) — do not thrash status.
        if !matches!(s.status, PipelineStatus::WaitingForStable { .. }) && s.auto_running {
            s.status = PipelineStatus::Capturing;
        }
        return;
    }

    info!(%reason, "clearing stale overlay");
    s.latest_ocr_blocks.clear();
    s.latest_ocr_text.clear();
    s.latest_translated_blocks.clear();
    s.latest_translated_text.clear();
    s.can_retry_translate = false;
    if s.auto_running {
        s.status = PipelineStatus::Capturing;
    } else {
        s.status = PipelineStatus::Idle;
    }
    drop(s);

    // Drop hysteresis copies of vanished text + allow re-translate on return.
    persist.reset();
    gate.reset_all();
    *ctx.last_translated_fp = None;
    *ctx.last_page = None;

    if let Some(o) = ctx.overlay
        && let Err(e) = o.clear()
    {
        warn!(error = %e, "failed to clear overlay");
    }
}

fn raw_empty_grace(state: &SharedState) -> Duration {
    let ms = state.read().config.ocr.block_max_miss_ms.max(1);
    Duration::from_millis(ms)
}

/// Expire captions after raw OCR went empty, even when capture stops producing frames.
fn maybe_expire_raw_empty(
    ctx: &mut TranslateCtx<'_>,
    gate: &mut StabilityGate,
    persist: &mut BlockPersistenceFilter,
    raw_empty_since: &mut Option<Instant>,
) {
    let Some(since) = *raw_empty_since else {
        return;
    };
    if since.elapsed() < raw_empty_grace(ctx.state) {
        return;
    }
    *raw_empty_since = None;
    clear_stale_overlay(ctx, gate, persist, "raw OCR empty grace elapsed (no new frames)");
}

/// Rebuild displayed translations from current OCR.
///
/// Match order:
/// 1. Exact normalized source text
/// 2. Same region (center + IoU) — OCR thrash often flips trailing glyphs so the
///    string no longer equals the last translated source, but the caption should
///    stay put until a real re-translate.
///
/// Sticky geometry: keep the **previous caption box** (position + size) so OCR
/// thrash cannot walk the overlay around. Detector width/center swings (trailing
/// glyphs) used to re-center the frozen-size box and look like the caption
/// "ran away". Fresh geometry comes only from `finish_translate` after a real
/// re-translate, or when the capture surface is resized.
///
/// When `content_resized` is true, adopt OCR boxes fully so coordinates match
/// the new frame.
fn remap_translations_to_ocr(translated: &[TranslatedBlock], ocr: &[OcrBlock], content_resized: bool) -> Vec<TranslatedBlock> {
    let mut used = vec![false; translated.len()];
    let mut out = Vec::new();

    for (i, ob) in ocr.iter().enumerate() {
        let key = normalize_match_text(&ob.text);
        if key.is_empty() {
            continue;
        }

        let text_hit = translated.iter().enumerate().find_map(|(ti, t)| {
            if used[ti] {
                return None;
            }
            if normalize_match_text(&t.source) == key { Some(ti) } else { None }
        });

        let spatial_hit = text_hit.or_else(|| best_spatial_translation(translated, &used, ob.bbox));

        let Some(ti) = spatial_hit else {
            continue;
        };
        used[ti] = true;
        let tb = &translated[ti];
        // Sticky: never follow OCR geometry. Resize path is the only exception.
        let bbox = if content_resized { ob.bbox } else { tb.bbox };
        out.push(TranslatedBlock {
            id: i as u32,
            // Keep the last translated source/translation; OCR string may thrash.
            source: tb.source.clone(),
            translation: tb.translation.clone(),
            confidence: ob.confidence,
            bbox,
            source_lines: tb.source_lines.max(1),
        });
    }
    out
}

/// Prefer a prior translation whose box still overlaps this OCR hit.
fn best_spatial_translation(translated: &[TranslatedBlock], used: &[bool], bbox: translator_core::Rect) -> Option<usize> {
    let (cx, cy) = bbox.center();
    let mut best: Option<(usize, f32)> = None;
    for (ti, t) in translated.iter().enumerate() {
        if used[ti] {
            continue;
        }
        let (tcx, tcy) = t.bbox.center();
        let dx = (cx - tcx).abs();
        let dy = (cy - tcy).abs();
        let max_dx = (bbox.width.max(t.bbox.width) * 0.55).max(20.0);
        let max_dy = (bbox.height.max(t.bbox.height) * 0.75).max(14.0);
        if dx > max_dx || dy > max_dy {
            continue;
        }
        let iou = bbox.iou(t.bbox);
        // Require meaningful overlap so neighboring lines do not steal captions.
        if iou < 0.20 {
            continue;
        }
        let score = iou * 3.0 + (1.0 - (dx / max_dx).clamp(0.0, 1.0)) + (1.0 - (dy / max_dy).clamp(0.0, 1.0));
        if best.map(|(_, s)| score > s).unwrap_or(true) {
            best = Some((ti, score));
        }
    }
    best.map(|(i, _)| i)
}

fn normalize_match_text(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// True when the remapped overlay set differs in count, text, or bbox.
///
/// Match by source+translation (not zip order): OCR reorder must not force a
/// repaint that re-runs label layout and looks like the captions moved.
fn translated_geometry_changed(previous: &[TranslatedBlock], remapped: &[TranslatedBlock]) -> bool {
    if previous.len() != remapped.len() {
        return true;
    }
    for b in remapped {
        let key = normalize_match_text(&b.source);
        let Some(a) = previous
            .iter()
            .find(|p| normalize_match_text(&p.source) == key && p.translation == b.translation)
        else {
            return true;
        };
        if a.bbox != b.bbox {
            return true;
        }
    }
    false
}

fn run_ocr_auto(
    ctx: &mut TranslateCtx<'_>,
    engine: &mut OcrEngine,
    mut filters: OcrFilters<'_>,
    timers: OcrTimers<'_>,
    frame: &CapturedFrame,
) {
    {
        let mut s = ctx.state.write();
        if s.translate_in_flight || matches!(s.status, PipelineStatus::Translating) {
            return;
        }
        // OCR can take hundreds of ms; avoid clobbering "waiting / overlay" so the
        // UI does not look stuck on "Running OCR" while text is already stable.
        if !matches!(s.status, PipelineStatus::WaitingForStable { .. } | PipelineStatus::OverlayActive) {
            s.status = PipelineStatus::RunningOcr;
        }
    }

    let ocr_start = Instant::now();
    let raw = match engine.recognize_rgba(frame.width, frame.height, &frame.rgba) {
        Ok(b) => b,
        Err(e) => {
            error!(error = %e, "OCR failed");
            ctx.state.write().set_error(format!("OCR: {e}"));
            return;
        }
    };
    let ocr_ms = ocr_start.elapsed().as_millis() as u64;
    {
        let mut s = ctx.state.write();
        s.last_ocr_ms = Some(ocr_ms);
        s.last_ocr_block_count = raw.len() as u32;
    }
    info!(ocr_ms, blocks = raw.len(), frame = frame.sequence, "OCR frame complete");

    // Persistence keeps vanished text for a short grace (icons/jitter). That is
    // useful for the stability gate, but must NOT keep ghost text "alive" for the
    // gate after the screen is actually blank — otherwise captions stick forever
    // when capture stops sending frames on a static empty view.
    let durable = filters.persist.filter(raw.clone());

    if raw.is_empty() {
        *timers.raw_content_since = None;
        *timers.remap_miss_since = None;
        let since = timers.raw_empty_since.get_or_insert_with(Instant::now);
        let grace = raw_empty_grace(ctx.state);
        if durable.is_empty() || since.elapsed() >= grace {
            *timers.raw_empty_since = None;
            // Full reset including persistence tracks — screen is blank.
            clear_stale_overlay(ctx, filters.gate, filters.persist, "raw OCR empty");
        }
        // During grace: leave the last overlay up, but do not feed hysteresis
        // copies back into the gate as if the text were still on screen.
        return;
    }
    *timers.raw_empty_since = None;
    let content_since = *timers.raw_content_since.get_or_insert_with(Instant::now);
    let max_unstable_ms = ctx.state.read().config.ocr.max_unstable_ms;
    let content_elapsed = content_since.elapsed();
    let force_unstable = max_unstable_ms > 0 && content_elapsed >= Duration::from_millis(max_unstable_ms);

    // Prefer durable (linger-filtered) blocks. If OCR never settles long enough
    // for persistence, after max_unstable force the latest raw reading through
    // so thrash still translates instead of spinning on Capturing forever.
    let (blocks, forced_raw) = if !durable.is_empty() {
        (durable, false)
    } else if force_unstable {
        info!(elapsed_ms = content_elapsed.as_millis() as u64, raw = raw.len(), "OCR thrash — forcing raw blocks past persistence");
        (raw.clone(), true)
    } else {
        // Still waiting for linger / force. Keep OCR preview, do not wipe an
        // existing overlay every frame (that made thrash look like "no translate").
        let mut s = ctx.state.write();
        s.latest_ocr_blocks = raw;
        s.latest_ocr_text = OcrEngine::blocks_to_text(&s.latest_ocr_blocks);
        s.can_retry_translate = false;
        s.status = PipelineStatus::WaitingForStable {
            elapsed_ms: content_elapsed.as_millis() as u64,
        };
        return;
    };

    let text = OcrEngine::blocks_to_text(&blocks);
    let fp = OcrFingerprint::from_blocks(&blocks);

    match filters.gate.observe(fp) {
        StabilityOutcome::Changed => {
            // Keep previous captions while the new page settles. Immediate clear
            // made every OCR blip look like the overlay vanished.
            apply_sticky_overlay(ctx, &mut filters, timers.remap_miss_since, &blocks, &text, frame.width, frame.height);
            let mut s = ctx.state.write();
            s.can_retry_translate = false;
            s.status = PipelineStatus::WaitingForStable { elapsed_ms: 0 };
        }
        StabilityOutcome::Waiting { elapsed_ms } => {
            apply_sticky_overlay(ctx, &mut filters, timers.remap_miss_since, &blocks, &text, frame.width, frame.height);
            let mut s = ctx.state.write();
            s.status = PipelineStatus::WaitingForStable { elapsed_ms };
        }
        StabilityOutcome::Ready { elapsed_ms, fingerprint } => {
            info!(blocks = blocks.len(), elapsed_ms, forced_raw, ?fingerprint, "OCR stable — translating");
            // Stick old translations (frozen size) until HTTP returns.
            apply_sticky_overlay(ctx, &mut filters, timers.remap_miss_since, &blocks, &text, frame.width, frame.height);
            // Fresh content session for force-raw after this emit settles.
            *timers.raw_content_since = Some(Instant::now());
            let page = PendingPage {
                blocks: blocks.clone(),
                source_text: text.clone(),
                fingerprint,
                content_width: frame.width,
                content_height: frame.height,
            };
            *ctx.last_page = Some(page.clone());
            {
                let mut s = ctx.state.write();
                s.can_retry_translate = true;
            }
            start_translate(ctx, page, false);
        }
        StabilityOutcome::AlreadyEmitted { .. } => {
            apply_sticky_overlay(ctx, &mut filters, timers.remap_miss_since, &blocks, &text, frame.width, frame.height);
            if ctx.state.read().auto_running {
                let mut s = ctx.state.write();
                if !s.latest_translated_blocks.is_empty() {
                    s.status = PipelineStatus::OverlayActive;
                }
            }
        }
    }
}

/// Sticky remap + delayed clear when nothing maps for a grace period.
fn apply_sticky_overlay(
    ctx: &mut TranslateCtx<'_>,
    filters: &mut OcrFilters<'_>,
    remap_miss_since: &mut Option<Instant>,
    blocks: &[OcrBlock],
    text: &str,
    frame_w: u32,
    frame_h: u32,
) {
    let content_resized = ctx
        .last_page
        .as_ref()
        .map(|p| p.content_width != frame_w || p.content_height != frame_h)
        .unwrap_or(true);

    let (had_translated, remapped, geometry_changed) = {
        let s = ctx.state.read();
        let had = !s.latest_translated_blocks.is_empty();
        let remapped = remap_translations_to_ocr(&s.latest_translated_blocks, blocks, content_resized);
        let geometry_changed = translated_geometry_changed(&s.latest_translated_blocks, &remapped);
        (had, remapped, geometry_changed)
    };

    {
        let mut s = ctx.state.write();
        s.latest_ocr_blocks = blocks.to_vec();
        s.latest_ocr_text = text.to_string();
    }

    if remapped.is_empty() {
        if had_translated {
            let since = remap_miss_since.get_or_insert_with(Instant::now);
            let grace = raw_empty_grace(ctx.state);
            if since.elapsed() >= grace {
                *remap_miss_since = None;
                clear_stale_overlay(ctx, filters.gate, filters.persist, "sticky remap empty past grace (layout gone)");
            }
        }
        // Keep last captions as-is during short thrash / partial miss.
        return;
    }

    *remap_miss_since = None;

    // Sticky freezes boxes: thrash should not push set_blocks (avoids re-layout).
    if !geometry_changed && !content_resized {
        return;
    }

    let translated_text = blocks_to_translated_text(&remapped);
    if let Some(o) = ctx.overlay {
        if let Some(hwnd) = ctx.state.read().target_hwnd {
            let _ = o.attach(hwnd);
        }
        if let Err(e) = o.set_blocks(remapped.clone(), frame_w, frame_h) {
            warn!(error = %e, "failed to refresh overlay bboxes");
        }
    }
    if let Some(page) = ctx.last_page.as_mut() {
        page.content_width = frame_w;
        page.content_height = frame_h;
    }
    let mut s = ctx.state.write();
    s.latest_translated_blocks = remapped;
    s.latest_translated_text = translated_text;
}

fn run_ocr_manual(ctx: &mut TranslateCtx<'_>, engine: &mut OcrEngine, gate: &mut StabilityGate, frame: &CapturedFrame) {
    {
        let mut s = ctx.state.write();
        s.status = PipelineStatus::RunningOcr;
    }

    let ocr_start = Instant::now();
    let blocks = match engine.recognize_rgba(frame.width, frame.height, &frame.rgba) {
        Ok(b) => b,
        Err(e) => {
            error!(error = %e, "manual OCR failed");
            ctx.state.write().set_error(format!("OCR: {e}"));
            return;
        }
    };
    let ocr_ms = ocr_start.elapsed().as_millis() as u64;
    {
        let mut s = ctx.state.write();
        s.last_ocr_ms = Some(ocr_ms);
        s.last_ocr_block_count = blocks.len() as u32;
    }

    let text = OcrEngine::blocks_to_text(&blocks);
    let fp = OcrFingerprint::from_blocks(&blocks);
    let _ = gate.force_emit(fp);

    info!(ocr_ms, blocks = blocks.len(), "manual OCR complete — translating");
    let page = PendingPage {
        blocks: blocks.clone(),
        source_text: text.clone(),
        fingerprint: fp,
        content_width: frame.width,
        content_height: frame.height,
    };
    *ctx.last_page = Some(page.clone());
    {
        let mut s = ctx.state.write();
        s.latest_ocr_blocks = blocks;
        s.latest_ocr_text = text;
        s.can_retry_translate = true;
    }
    // Manual always forces a new API call (user intent).
    start_translate(ctx, page, true);
}

fn start_translate(ctx: &mut TranslateCtx<'_>, page: PendingPage, force: bool) {
    if ctx.inflight.is_some() {
        return;
    }

    {
        let mut s = ctx.state.write();
        s.latest_ocr_blocks = page.blocks.clone();
        s.latest_ocr_text = page.source_text.clone();
    }

    if page.blocks.is_empty() || page.source_text.trim().is_empty() {
        *ctx.last_translated_fp = None;
        *ctx.last_page = None;
        let mut s = ctx.state.write();
        s.latest_translated_text.clear();
        s.latest_translated_blocks.clear();
        s.can_retry_translate = false;
        s.translate_in_flight = false;
        if s.auto_running {
            s.status = PipelineStatus::Capturing;
        } else {
            s.status = PipelineStatus::Idle;
        }
        drop(s);
        // Manual / empty-page path must also wipe the live overlay window.
        if let Some(o) = ctx.overlay
            && let Err(e) = o.clear()
        {
            warn!(error = %e, "failed to clear overlay on empty page");
        }
        return;
    }

    // Skip identical content unless forced (saves API calls).
    if !force && *ctx.last_translated_fp == Some(page.fingerprint) {
        info!(?page.fingerprint, "skip translate — content unchanged");
        let mut s = ctx.state.write();
        s.translate_in_flight = false;
        if !s.latest_translated_blocks.is_empty() {
            s.status = PipelineStatus::OverlayActive;
        } else if s.auto_running {
            s.status = PipelineStatus::Capturing;
        }
        return;
    }

    let (api, tcfg) = {
        let s = ctx.state.read();
        (s.config.api.clone(), s.config.translation.clone())
    };
    ctx.client.update_api(api);

    // Prepare conversation on the pipeline thread (shared mutable state).
    ctx.conversation
        .compress_if_needed(tcfg.conversation_max_turns, tcfg.history_max_items);
    ctx.conversation.ensure_system(translator_translate::default_system_prompt(&tcfg));
    let user = translator_translate::user_payload_from_blocks(&page.blocks);
    ctx.conversation.push_user(user);
    let messages_len_after_user = ctx.conversation.messages.len();
    let messages = ctx.conversation.messages.clone();

    let cancel = CancellationToken::new();
    let cancel_job = cancel.clone();
    let client_clone = ctx.client.clone();
    let (tx, rx) = oneshot::channel();

    {
        let mut s = ctx.state.write();
        s.status = PipelineStatus::Translating;
        s.translate_in_flight = true;
        s.last_error = None;
        s.can_retry_translate = true;
    }

    ctx.rt.spawn(async move {
        let result = client_clone.chat_completions_with_retry(&messages, &cancel_job).await;
        let _ = tx.send(TranslateJobResult {
            result,
            messages_len_after_user,
        });
    });

    *ctx.inflight = Some(InflightTranslate {
        cancel,
        rx,
        fingerprint: page.fingerprint,
        blocks: page.blocks,
        source_text: page.source_text,
        content_width: page.content_width,
        content_height: page.content_height,
    });
}

fn finish_translate(
    state: &SharedState,
    conversation: &mut Conversation,
    overlay: Option<&OverlayController>,
    job: InflightTranslate,
    job_result: TranslateJobResult,
    last_translated_fp: &mut Option<OcrFingerprint>,
) {
    // Roll back user turn on failure so Retry can re-push cleanly.
    let rollback_user = |conversation: &mut Conversation| {
        if conversation.messages.len() == job_result.messages_len_after_user
            && conversation.messages.last().map(|m| m.role == "user").unwrap_or(false)
        {
            conversation.messages.pop();
        }
    };

    match job_result.result {
        Ok(content) => {
            match merge_translations(&job.blocks, &content) {
                Ok(translated) => {
                    conversation.push_assistant(&content);
                    let translated_text = blocks_to_translated_text(&translated);
                    info!(blocks = translated.len(), turns = conversation.turn_count, "translation complete");
                    *last_translated_fp = Some(job.fingerprint);
                    apply_translated(state, overlay, job.source_text, translated, translated_text, job.content_width, job.content_height);
                }
                Err(e) => {
                    error!(error = %e, "failed to merge translation");
                    // Drop the pending user turn so Retry re-sends a clean request
                    // (do not store invalid model JSON as assistant context).
                    rollback_user(conversation);
                    let mut s = state.write();
                    s.translate_in_flight = false;
                    s.can_retry_translate = true;
                    s.set_error(format!("translate parse: {e}"));
                }
            }
        }
        Err(e) if e.is_cancelled() => {
            info!("translation cancelled");
            rollback_user(conversation);
            let mut s = state.write();
            s.translate_in_flight = false;
            s.can_retry_translate = true;
            if s.auto_running {
                s.status = PipelineStatus::Capturing;
            } else {
                s.status = PipelineStatus::Cancelled;
            }
        }
        Err(e) => {
            error!(error = %e, "translation failed");
            rollback_user(conversation);
            let mut s = state.write();
            s.translate_in_flight = false;
            s.can_retry_translate = true;
            s.set_error(format!("translate: {e}"));
        }
    }
}

fn apply_translated(
    state: &SharedState,
    overlay: Option<&OverlayController>,
    source_text: String,
    translated: Vec<TranslatedBlock>,
    translated_text: String,
    content_width: u32,
    content_height: u32,
) {
    if let Some(o) = overlay {
        if let Some(hwnd) = state.read().target_hwnd {
            let _ = o.attach(hwnd);
        }
        if let Err(e) = o.set_blocks(translated.clone(), content_width, content_height) {
            warn!(error = %e, "failed to update overlay");
        }
    }

    let mut s = state.write();
    s.latest_translated_blocks = translated.clone();
    s.latest_translated_text = translated_text.clone();
    s.push_history(source_text, translated_text, translated);
    s.translate_in_flight = false;
    s.can_retry_translate = true;
    s.last_error = None;
    s.status = PipelineStatus::OverlayActive;
}

fn save_preview(frame: &CapturedFrame) -> Result<PathBuf, String> {
    let thumb = frame.thumbnail(480);
    let png = thumb.to_png_bytes().map_err(|e| e.to_string())?;
    let dir = translator_core::exe_dir().map_err(|e| e.to_string())?;
    let path = dir.join("preview_last.png");
    std::fs::write(&path, png).map_err(|e| e.to_string())?;
    Ok(path)
}

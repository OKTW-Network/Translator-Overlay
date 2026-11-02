//! Auto / manual OCR paths, sticky overlay, and raw-empty expiry.

use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

use rust_i18n::t;
use tokio::task::JoinError;
use tracing::{debug, error, info};
use translator_capture::CapturedFrame;
use translator_core::{NormRect, OcrBlock, PipelineStatus, Rect, normalize_ocr_text};
use translator_ocr::{OcrError, OcrFingerprint, StabilityOutcome};
use translator_overlay::OverlayCommand;

use crate::pipeline::worker::{LastRawOcr, OcrJob, PendingPage, Pipeline, remap_translations_to_ocr};

impl Pipeline {
    /// Clear captions / overlay only. Keeps OCR preview, gate, persist, and
    /// `last_page` so a mid-page layout change does not restart stability.
    pub(crate) fn clear_translated_captions_only(&mut self, reason: &str) {
        if self.captions.is_empty() {
            return;
        }
        info!(%reason, "clearing translated captions");
        self.set_captions(Vec::new());
        self.remap_miss_since = None;
        self.send_overlay(OverlayCommand::Clear);
    }

    /// Drop stale captions so they do not stick on the overlay, and forget
    /// persistence tracks so the same page can re-translate if it reappears.
    /// Use for true blank screens — not for layout remap misses.
    pub(crate) fn clear_stale_overlay(&mut self, reason: &str) {
        let translating = self.inflight.is_some();
        if self.captions.is_empty() && self.ocr_blocks.is_empty() {
            // Still warming up (icons/flicker filtered out) — do not thrash status.
            let mut s = self.state.write();
            if !translating && !matches!(s.status, PipelineStatus::WaitingForStable { .. }) && s.auto_running {
                s.status = PipelineStatus::Capturing;
            }
            return;
        }

        info!(%reason, "clearing stale overlay");
        self.set_ocr_blocks(Vec::new());
        self.set_captions(Vec::new());
        // A pending translation keeps its status.
        if !translating {
            let mut s = self.state.write();
            s.status = if s.auto_running {
                PipelineStatus::Capturing
            } else {
                PipelineStatus::Idle
            };
        }

        // Drop hysteresis copies of vanished text + allow re-translate on return.
        self.persist.reset();
        self.gate.reset_all();
        self.last_page = None;
        self.remap_miss_since = None;
        self.send_overlay(OverlayCommand::Clear);
    }

    fn raw_empty_grace(&self) -> Duration {
        let ms = self.state.read().config.ocr.hold_ms.max(1);
        Duration::from_millis(ms)
    }

    /// Expire captions after raw OCR went empty, even when capture stops producing frames.
    pub(crate) fn maybe_expire_raw_empty(&mut self) {
        let Some(since) = self.raw_empty_since else {
            return;
        };
        if since.elapsed() < self.raw_empty_grace() {
            return;
        }
        self.raw_empty_since = None;
        self.clear_stale_overlay("raw OCR empty grace elapsed (no new frames)");
    }

    /// Expire sticky captions when remap has been empty past grace (no new OCR needed).
    pub(crate) fn maybe_expire_remap_miss(&mut self) {
        let Some(since) = self.remap_miss_since else {
            return;
        };
        if since.elapsed() < self.raw_empty_grace() {
            return;
        }
        self.remap_miss_since = None;
        self.clear_translated_captions_only("sticky remap empty past grace (layout gone)");
        // Captions are gone: let the same page re-translate if it returns.
        self.gate.reset_all();
        self.last_page = None;
    }

    /// Run pending overlay expiry. Call every capture interval, including skipped OCR.
    pub(crate) fn expire_pending(&mut self) {
        self.maybe_expire_raw_empty();
        self.maybe_expire_remap_miss();
    }

    pub(crate) fn run_ocr_auto(&mut self, frame: &CapturedFrame) {
        let rects = self.ocr_pixel_regions(frame.width, frame.height);

        // Reuse the raw blocks when the OCR scope is unchanged: skip only the
        // inference; consumption still runs every tick.
        if let Some(cached) = self.last_raw_ocr.as_mut()
            && cached.same_content(frame, &rects)
        {
            // New sequence, identical scope pixels (continuously redrawing
            // target): adopt the sequence so stale ticks skip at the
            // drive_capture gate.
            cached.sequence = frame.sequence;
            debug!(frame = frame.sequence, blocks = cached.blocks.len(), "OCR content cache hit — skipping inference");
            let blocks = cached.blocks.clone();
            self.consume_raw_ocr(blocks, frame.width, frame.height);
            return;
        }
        self.start_ocr(frame, &rects, false);
    }

    /// Start OCR on `frame` beside the loop; [`Self::finish_ocr`] takes the result.
    pub(crate) fn start_ocr(&mut self, frame: &CapturedFrame, rects: &[Rect], manual: bool) {
        let Some(engine) = self.engine.clone() else {
            return;
        };
        let (width, height, rgba, regions) = (frame.width, frame.height, frame.rgba.clone(), rects.to_vec());
        let handle = tokio::spawn(async move { engine.recognize_rgba_regions(width, height, &rgba, &regions).await });
        self.ocr_job = Some(OcrJob {
            handle,
            started: Instant::now(),
            sequence: frame.sequence,
            width,
            height,
            scope: LastRawOcr::pack_scope(frame, rects),
            manual,
            stale: false,
        });
    }

    /// Record the finished OCR job, key its blocks to the frame, and consume them.
    pub(crate) fn finish_ocr(&mut self, result: Result<Result<Vec<OcrBlock>, OcrError>, JoinError>) {
        let Some(job) = self.ocr_job.take() else {
            return;
        };
        if job.stale {
            debug!(frame = job.sequence, "dropping OCR result from before a reset");
        } else {
            match result.unwrap_or_else(|e| Err(OcrError::Other(format!("OCR task: {e}")))) {
                Ok(blocks) => {
                    let ocr_ms = job.started.elapsed().as_millis() as u64;
                    {
                        let mut s = self.state.write();
                        s.last_ocr_ms = Some(ocr_ms);
                        s.last_ocr_block_count = blocks.len() as u32;
                    }
                    debug!(ocr_ms, blocks = blocks.len(), frame = job.sequence, "OCR frame complete");
                    self.last_raw_ocr = Some(LastRawOcr {
                        sequence: job.sequence,
                        width: job.width,
                        height: job.height,
                        scope: job.scope,
                        blocks: blocks.clone(),
                    });
                    if job.manual {
                        self.translate_manual(blocks, job.width, job.height);
                    } else if !self.manual_queued && !self.state.read().capture_paused {
                        // Otherwise only cache the frame: a queued manual capture must not find an
                        // auto translate started in front of it, and Paused keeps its status.
                        self.consume_raw_ocr(blocks, job.width, job.height);
                    }
                }
                Err(e) => {
                    error!(error = %e, "OCR failed");
                    self.state.write().set_error(t!("err.ocr", error = e.to_string()));
                }
            }
        }
        if std::mem::take(&mut self.manual_queued) {
            self.manual_capture();
        }
    }

    /// Status guard + persist / stability gate / remap. Runs on every capture
    /// tick, including cache hits (`raw` / dims taken from the cache).
    pub(crate) fn consume_raw_ocr(&mut self, raw: Vec<OcrBlock>, frame_w: u32, frame_h: u32) {
        // A pending translation runs the caption rules below but leaves status and the
        // stability gate alone; show_captions maps its paints onto ocr_blocks.
        let translating = self.inflight.is_some();
        if !translating {
            let mut s = self.state.write();
            // OCR can take hundreds of ms; avoid clobbering "waiting / overlay" so the
            // UI does not look stuck on "Running OCR" while text is already stable.
            // An already-emitted page never leaves RunningOcr after a failed translate, so keep Error.
            if !matches!(s.status, PipelineStatus::WaitingForStable { .. } | PipelineStatus::OverlayActive | PipelineStatus::Error { .. }) {
                s.status = PipelineStatus::RunningOcr;
            }
        }

        let content_resized = self.capture_content_resized(frame_w, frame_h);
        if content_resized {
            // Old tracks are in the previous pixel space; hysteresis would paint
            // those boxes onto the new content size and freeze captions.
            self.persist.reset();
        }

        // Persistence keeps vanished text for a short grace (icons/jitter). That is
        // useful for the stability gate, but must NOT keep ghost text "alive" for the
        // gate after the screen is actually blank — otherwise captions stick forever
        // when capture stops sending frames on a static empty view.
        let durable = self.persist.filter(raw.clone());

        if raw.is_empty() {
            self.raw_content_since = None;
            self.remap_miss_since = None;
            let grace = self.raw_empty_grace();
            let since = *self.raw_empty_since.get_or_insert_with(Instant::now);
            if durable.is_empty() || since.elapsed() >= grace {
                self.raw_empty_since = None;
                // Full reset including persistence tracks — screen is blank.
                self.clear_stale_overlay("raw OCR empty");
            }
            // During grace: leave the last overlay up, but do not feed hysteresis
            // copies back into the gate as if the text were still on screen.
            return;
        }
        self.raw_empty_since = None;
        let content_since = *self.raw_content_since.get_or_insert_with(Instant::now);

        // Prefer durable (settle-filtered) blocks; thrashing text reaches it through
        // the filter's own max_wait force-confirm. After a capture-surface resize,
        // persist was reset — use raw immediately so sticky remap can adopt boxes in
        // the new pixel space.
        let blocks = if content_resized {
            raw
        } else if !durable.is_empty() {
            durable
        } else {
            // Nothing settled (or only unsettled noise left). Keep the OCR preview
            // and let captions whose source vanished expire after the hold grace.
            self.apply_sticky_overlay(&[], frame_w, frame_h);
            self.set_ocr_blocks(raw);
            if !translating {
                self.state.write().status = PipelineStatus::WaitingForStable {
                    elapsed_ms: content_since.elapsed().as_millis() as u64,
                };
            }
            return;
        };
        if translating {
            // The gate would emit a new page that start_translate drops while busy.
            self.apply_sticky_overlay(&blocks, frame_w, frame_h);
            return;
        }

        let fp = OcrFingerprint::from_blocks(&blocks);
        // Lines vanishing from the emitted page only drop their captions (sticky
        // remap is by source text); re-translating the remainder flickered.
        let subset_of_last = self.last_page.as_ref().is_some_and(|p| {
            let prior: HashSet<String> = p.blocks.iter().map(|b| normalize_ocr_text(&b.text)).collect();
            blocks.iter().all(|b| prior.contains(&normalize_ocr_text(&b.text)))
        });
        let outcome = if subset_of_last {
            // Remap drops a vanished line's caption: shrink the emitted page so the
            // line's return reaches the gate as new content, not a captionless sticky hit.
            if let Some(p) = self.last_page.as_mut()
                && blocks.len() < p.blocks.len()
            {
                p.blocks = blocks.clone();
                let _ = self.gate.force_emit(fp);
            }
            StabilityOutcome::AlreadyEmitted { fingerprint: fp }
        } else {
            self.gate.observe(fp)
        };

        match outcome {
            StabilityOutcome::Waiting { elapsed_ms } => {
                // Only before the first emit or after a gate reset: captions whose
                // source is still on screen stay, the rest drop.
                self.apply_sticky_overlay(&blocks, frame_w, frame_h);
                self.state.write().status = PipelineStatus::WaitingForStable { elapsed_ms };
            }
            StabilityOutcome::Ready { elapsed_ms, fingerprint } => {
                info!(blocks = blocks.len(), elapsed_ms, ?fingerprint, "OCR stable — translating");
                // New content about to translate — do not show prior-page text
                // over the new scene for the whole network latency window.
                self.clear_translated_captions_only("new page ready for translate");
                // Waiting status counts from this emit.
                self.raw_content_since = Some(Instant::now());
                let page = PendingPage {
                    blocks,
                    content_width: frame_w,
                    content_height: frame_h,
                };
                self.last_page = Some(page.clone());
                self.start_translate(page, false);
            }
            StabilityOutcome::AlreadyEmitted { .. } => {
                // Persist-frozen page: keep frozen captions while durable OCR
                // still matches the last sources. Raw jitter must not hide them.
                self.apply_sticky_overlay(&blocks, frame_w, frame_h);
                if !self.captions.is_empty() && self.state.read().auto_running {
                    self.state.write().status = PipelineStatus::OverlayActive;
                }
            }
        }
    }

    /// Sticky remap + delayed caption-only clear when nothing maps for a grace period.
    fn apply_sticky_overlay(&mut self, blocks: &[OcrBlock], frame_w: u32, frame_h: u32) {
        let content_resized = self.capture_content_resized(frame_w, frame_h) || self.last_page.is_none();
        let remapped = remap_translations_to_ocr(&self.captions, blocks, content_resized);
        self.set_ocr_blocks(blocks.to_vec());

        if remapped.is_empty() {
            let had_captions = !self.captions.is_empty();
            if had_captions && !blocks.is_empty() {
                // Durable OCR changed (persist adopted a new string). Raw
                // single-frame jitter must not reach here — persist still
                // emits the last source until the new reading lingers.
                self.clear_translated_captions_only("source no longer matches captions");
                return;
            }
            if had_captions {
                let _ = self.remap_miss_since.get_or_insert_with(Instant::now);
                self.maybe_expire_remap_miss();
            }
            // Empty OCR: keep last captions during short miss / vanish grace.
            return;
        }

        self.remap_miss_since = None;
        if self.paint_captions(remapped, frame_w, frame_h)
            && let Some(page) = self.last_page.as_mut()
        {
            page.content_width = frame_w;
            page.content_height = frame_h;
        }
    }

    /// Translate a manual capture's OCR now, bypassing the stability wait.
    fn translate_manual(&mut self, blocks: Vec<OcrBlock>, frame_w: u32, frame_h: u32) {
        let _ = self.gate.force_emit(OcrFingerprint::from_blocks(&blocks));
        // Re-learn every track: frozen (possibly misread) text would re-emit over this translate.
        self.persist.reset();

        info!(blocks = blocks.len(), "manual OCR complete — translating");
        let page = PendingPage {
            blocks,
            content_width: frame_w,
            content_height: frame_h,
        };
        self.last_page = Some(page.clone());
        self.clear_translated_captions_only("manual capture");
        // Manual always forces a new API call (user intent).
        self.start_translate(page, true);
    }

    fn capture_content_resized(&self, frame_w: u32, frame_h: u32) -> bool {
        self.last_page
            .as_ref()
            .is_some_and(|p| p.content_width != frame_w || p.content_height != frame_h)
    }

    pub(crate) fn ocr_pixel_regions(&self, frame_w: u32, frame_h: u32) -> Vec<Rect> {
        self.state
            .read()
            .ocr_regions
            .iter()
            .copied()
            .filter_map(NormRect::sanitize)
            .map(|n| n.to_pixel(frame_w, frame_h))
            .collect()
    }

    pub(crate) fn apply_ocr_regions(&mut self, regions: Vec<NormRect>) {
        let regions: Vec<NormRect> = regions.into_iter().filter_map(NormRect::sanitize).collect();
        {
            let mut s = self.state.write();
            s.ocr_regions = regions;
            s.region_select_draft.clear();
            s.region_select_active = false;
        }
        self.cancel_inflight();
        self.reset_ocr_session(false);
    }
}

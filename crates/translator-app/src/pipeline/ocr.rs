//! Auto / manual OCR paths, sticky overlay, and raw-empty expiry.

use std::time::{Duration, Instant};

use tracing::{debug, error, info, warn};
use translator_capture::CapturedFrame;
use translator_core::{NormRect, OcrBlock, PipelineStatus, Rect};
use translator_ocr::{OcrEngine, OcrFingerprint, StabilityOutcome};

use crate::pipeline::{
    remap::{remap_translations_to_ocr, translated_geometry_changed},
    worker::{LastRawOcr, PendingPage, Pipeline},
};

impl Pipeline {
    /// Clear captions / overlay only. Keeps OCR preview, gate, persist, and
    /// `last_page` so a mid-page layout change does not restart stability.
    pub(crate) fn clear_translated_captions_only(&mut self, reason: &str) {
        let had = {
            let s = self.state.read();
            !s.latest_translated_blocks.is_empty()
        };
        if !had {
            return;
        }
        info!(%reason, "clearing translated captions");
        {
            let mut s = self.state.write();
            s.latest_translated_blocks.clear();
        }
        self.last_translated_fp = None;
        self.remap_miss_since = None;
        if let Some(o) = self.overlay.as_ref()
            && let Err(e) = o.clear()
        {
            warn!(error = %e, "failed to clear overlay captions");
        }
    }

    /// Drop stale captions so they do not stick on the overlay, and forget
    /// fingerprints / persistence tracks so the same page can re-translate if it
    /// reappears. Use for true blank screens — not for layout remap misses.
    pub(crate) fn clear_stale_overlay(&mut self, reason: &str) {
        let mut s = self.state.write();
        let had_content = !s.latest_translated_blocks.is_empty() || !s.latest_ocr_blocks.is_empty();

        if !had_content {
            // Still warming up (icons/flicker filtered out) — do not thrash status.
            if !matches!(s.status, PipelineStatus::WaitingForStable { .. }) && s.auto_running {
                s.status = PipelineStatus::Capturing;
            }
            return;
        }

        info!(%reason, "clearing stale overlay");
        s.latest_ocr_blocks.clear();
        s.latest_translated_blocks.clear();
        if s.auto_running {
            s.status = PipelineStatus::Capturing;
        } else {
            s.status = PipelineStatus::Idle;
        }
        drop(s);

        // Drop hysteresis copies of vanished text + allow re-translate on return.
        self.persist.reset();
        self.gate.reset_all();
        self.last_translated_fp = None;
        self.last_page = None;
        self.remap_miss_since = None;

        if let Some(o) = self.overlay.as_ref()
            && let Err(e) = o.clear()
        {
            warn!(error = %e, "failed to clear overlay");
        }
    }

    fn raw_empty_grace(&self) -> Duration {
        let ms = self.state.read().config.ocr.block_max_miss_ms.max(1);
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
    }

    /// Run pending overlay expiry. Call every capture interval, including skipped OCR.
    pub(crate) fn expire_pending(&mut self) {
        self.maybe_expire_raw_empty();
        self.maybe_expire_remap_miss();
    }

    pub(crate) async fn run_ocr_auto(&mut self, frame: &CapturedFrame) {
        {
            let mut s = self.state.write();
            if s.translate_in_flight || s.status.is_translating() {
                return;
            }
            // OCR can take hundreds of ms; avoid clobbering "waiting / overlay" so the
            // UI does not look stuck on "Running OCR" while text is already stable.
            if !matches!(s.status, PipelineStatus::WaitingForStable { .. } | PipelineStatus::OverlayActive) {
                s.status = PipelineStatus::RunningOcr;
            }
        }

        // Reuse the raw blocks when the frame is unchanged (static content): skip
        // only the inference; everything below still runs every tick.
        let raw = match self.last_raw_ocr.as_ref().filter(|c| c.matches(frame)) {
            Some(cached) => {
                debug!(frame = frame.sequence, blocks = cached.blocks.len(), "OCR frame cache hit — skipping inference");
                cached.blocks.clone()
            }
            None => {
                let ocr_start = Instant::now();
                let regions = self.ocr_pixel_regions(frame.width, frame.height);
                let Some(engine) = self.engine.as_ref() else {
                    return;
                };
                let raw = match engine
                    .recognize_rgba_regions(frame.width, frame.height, &frame.rgba, &regions)
                    .await
                {
                    Ok(b) => b,
                    Err(e) => {
                        error!(error = %e, "OCR failed");
                        self.state.write().set_error(format!("OCR: {e}"));
                        return;
                    }
                };
                let ocr_ms = ocr_start.elapsed().as_millis() as u64;
                {
                    let mut s = self.state.write();
                    s.last_ocr_ms = Some(ocr_ms);
                    s.last_ocr_block_count = raw.len() as u32;
                }
                debug!(ocr_ms, blocks = raw.len(), frame = frame.sequence, "OCR frame complete");
                self.last_raw_ocr = Some(LastRawOcr {
                    sequence: frame.sequence,
                    width: frame.width,
                    height: frame.height,
                    blocks: raw.clone(),
                });
                raw
            }
        };

        let content_resized = self.capture_content_resized(frame.width, frame.height);
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
        let max_unstable_ms = self.state.read().config.ocr.max_unstable_ms;
        let content_elapsed = content_since.elapsed();
        let force_unstable = max_unstable_ms > 0 && content_elapsed >= Duration::from_millis(max_unstable_ms);

        // Prefer durable (linger-filtered) blocks. If OCR never settles long enough
        // for persistence, after max_unstable force the latest raw reading through
        // so thrash still translates instead of spinning on Capturing forever.
        // After a capture-surface resize, persist was reset — use raw immediately
        // so sticky remap can adopt boxes in the new pixel space.
        let (blocks, forced_raw) = if content_resized {
            (raw.clone(), false)
        } else if !durable.is_empty() {
            (durable, false)
        } else if force_unstable {
            info!(elapsed_ms = content_elapsed.as_millis() as u64, raw = raw.len(), "OCR thrash — forcing raw blocks past persistence");
            (raw.clone(), true)
        } else {
            // Still waiting for linger / force. Keep OCR preview, do not wipe an
            // existing overlay every frame (that made thrash look like "no translate").
            let mut s = self.state.write();
            s.latest_ocr_blocks = raw;
            s.status = PipelineStatus::WaitingForStable {
                elapsed_ms: content_elapsed.as_millis() as u64,
            };
            return;
        };

        let fp = OcrFingerprint::from_blocks(&blocks);

        match self.gate.observe(fp) {
            StabilityOutcome::Changed => {
                // Confirmed fingerprint switch: drop old captions so the wrong
                // language is not painted over a new scene while waiting for stable.
                {
                    let mut s = self.state.write();
                    s.latest_ocr_blocks = blocks.clone();
                    s.status = PipelineStatus::WaitingForStable { elapsed_ms: 0 };
                }
                self.clear_translated_captions_only("fingerprint changed");
            }
            StabilityOutcome::Waiting { elapsed_ms } => {
                // Settling a not-yet-emitted page: keep sticky only when most prior
                // captions still match (partial thrash); otherwise clear.
                self.apply_sticky_or_clear_low_hit(&blocks, frame.width, frame.height);
                let mut s = self.state.write();
                s.status = PipelineStatus::WaitingForStable { elapsed_ms };
            }
            StabilityOutcome::Ready { elapsed_ms, fingerprint } => {
                info!(blocks = blocks.len(), elapsed_ms, forced_raw, ?fingerprint, "OCR stable — translating");
                // New content about to translate — do not show prior-page text
                // over the new scene for the whole network latency window.
                {
                    let mut s = self.state.write();
                    s.latest_ocr_blocks = blocks.clone();
                }
                self.clear_translated_captions_only("new page ready for translate");
                // Fresh content session for force-raw after this emit settles.
                self.raw_content_since = Some(Instant::now());
                let page = PendingPage {
                    blocks: blocks.clone(),
                    source_text: OcrEngine::blocks_to_text(&blocks),
                    fingerprint,
                    content_width: frame.width,
                    content_height: frame.height,
                };
                self.last_page = Some(page.clone());
                self.start_translate(page, false);
            }
            StabilityOutcome::AlreadyEmitted { .. } => {
                // Persist-frozen page: keep frozen captions while durable OCR
                // still matches the last sources. Raw jitter must not hide them.
                self.apply_sticky_overlay(&blocks, frame.width, frame.height);
                if self.state.read().auto_running {
                    let mut s = self.state.write();
                    if !s.latest_translated_blocks.is_empty() {
                        s.status = PipelineStatus::OverlayActive;
                    }
                }
            }
        }
    }

    /// Sticky remap when hit-rate is high; clear captions on low coverage (page change).
    fn apply_sticky_or_clear_low_hit(&mut self, blocks: &[OcrBlock], frame_w: u32, frame_h: u32) {
        let prior_count = self.state.read().latest_translated_blocks.len();
        if prior_count == 0 {
            let mut s = self.state.write();
            s.latest_ocr_blocks = blocks.to_vec();
            return;
        }
        let content_resized = self.capture_content_resized(frame_w, frame_h) || self.last_page.is_none();
        let remapped = {
            let s = self.state.read();
            remap_translations_to_ocr(&s.latest_translated_blocks, blocks, content_resized)
        };
        // Low hit-rate ⇒ content largely replaced; hide old translations.
        let hit_rate = remapped.len() as f32 / prior_count as f32;
        if hit_rate < 0.5 {
            {
                let mut s = self.state.write();
                s.latest_ocr_blocks = blocks.to_vec();
            }
            self.clear_translated_captions_only("sticky hit-rate low on page change");
            return;
        }
        self.apply_sticky_overlay(blocks, frame_w, frame_h);
    }

    /// Sticky remap + delayed caption-only clear when nothing maps for a grace period.
    fn apply_sticky_overlay(&mut self, blocks: &[OcrBlock], frame_w: u32, frame_h: u32) {
        let content_resized = self.capture_content_resized(frame_w, frame_h) || self.last_page.is_none();

        let (had_translated, remapped, geometry_changed) = {
            let s = self.state.read();
            let had = !s.latest_translated_blocks.is_empty();
            let remapped = remap_translations_to_ocr(&s.latest_translated_blocks, blocks, content_resized);
            let geometry_changed = translated_geometry_changed(&s.latest_translated_blocks, &remapped);
            (had, remapped, geometry_changed)
        };

        {
            let mut s = self.state.write();
            s.latest_ocr_blocks = blocks.to_vec();
        }

        if remapped.is_empty() {
            if had_translated && !blocks.is_empty() {
                // Durable OCR changed (persist adopted a new string). Raw
                // single-frame jitter must not reach here — persist still
                // emits the last source until the new reading lingers.
                self.clear_translated_captions_only("source no longer matches captions");
                return;
            }
            if had_translated {
                let _ = self.remap_miss_since.get_or_insert_with(Instant::now);
                self.maybe_expire_remap_miss();
            }
            // Empty OCR: keep last captions during short miss / vanish grace.
            return;
        }

        self.remap_miss_since = None;

        // Unchanged geometry: skip set_blocks (avoids re-layout flicker).
        // Also skip when only the capture size changed but boxes are still in
        // the old pixel space — overlay host stretch keeps captions aligned
        // until remap adopts new-space boxes (`content_resized` + new OCR).
        if !geometry_changed {
            return;
        }

        if let Some(o) = self.overlay.as_ref() {
            if let Some(hwnd) = self.state.read().target_hwnd {
                let _ = o.attach(hwnd);
            }
            if let Err(e) = o.set_blocks(remapped.clone(), frame_w, frame_h) {
                warn!(error = %e, "failed to refresh overlay bboxes");
            }
        }
        if let Some(page) = self.last_page.as_mut() {
            page.content_width = frame_w;
            page.content_height = frame_h;
        }
        let mut s = self.state.write();
        s.latest_translated_blocks = remapped;
    }

    pub(crate) async fn run_ocr_manual(&mut self, frame: &CapturedFrame) {
        {
            let mut s = self.state.write();
            s.status = PipelineStatus::RunningOcr;
        }

        let ocr_start = Instant::now();
        let regions = self.ocr_pixel_regions(frame.width, frame.height);
        let Some(engine) = self.engine.as_ref() else {
            return;
        };
        let blocks = match engine
            .recognize_rgba_regions(frame.width, frame.height, &frame.rgba, &regions)
            .await
        {
            Ok(b) => b,
            Err(e) => {
                error!(error = %e, "manual OCR failed");
                self.state.write().set_error(format!("OCR: {e}"));
                return;
            }
        };
        let ocr_ms = ocr_start.elapsed().as_millis() as u64;
        {
            let mut s = self.state.write();
            s.last_ocr_ms = Some(ocr_ms);
            s.last_ocr_block_count = blocks.len() as u32;
        }
        // Manual capture always infers, but its result keys the frame for auto ticks.
        self.last_raw_ocr = Some(LastRawOcr {
            sequence: frame.sequence,
            width: frame.width,
            height: frame.height,
            blocks: blocks.clone(),
        });

        let text = OcrEngine::blocks_to_text(&blocks);
        let fp = OcrFingerprint::from_blocks(&blocks);
        let _ = self.gate.force_emit(fp);

        info!(ocr_ms, blocks = blocks.len(), "manual OCR complete — translating");
        let page = PendingPage {
            blocks: blocks.clone(),
            source_text: text,
            fingerprint: fp,
            content_width: frame.width,
            content_height: frame.height,
        };
        self.last_page = Some(page.clone());
        {
            let mut s = self.state.write();
            s.latest_ocr_blocks = blocks;
        }
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
        self.on_regions_changed();
    }

    pub(crate) fn on_regions_changed(&mut self) {
        self.cancel_inflight();
        self.gate.reset_all();
        self.persist.reset();
        self.last_translated_fp = None;
        self.last_page = None;
        self.last_raw_ocr = None;
        self.raw_empty_since = None;
        self.raw_content_since = None;
        self.remap_miss_since = None;
        {
            let mut s = self.state.write();
            s.latest_ocr_blocks.clear();
            s.latest_translated_blocks.clear();
            s.translate_in_flight = false;
        }
        if let Some(o) = self.overlay.as_ref() {
            let _ = o.clear();
        }
    }
}

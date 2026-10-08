//! Automatic and manual OCR, the sticky overlay, and caption expiry when raw OCR goes empty.

use std::time::{Duration, Instant};

use rust_i18n::t;
use tracing::{debug, error, info, warn};
use translator_capture::CapturedFrame;
use translator_core::{NormRect, OcrBlock, PipelineStatus, Rect};
use translator_ocr::{OcrFingerprint, StabilityOutcome};
use translator_overlay::OverlayCommand;

use crate::pipeline::worker::{LastRawOcr, PendingPage, Pipeline, remap_translations_to_ocr, translated_geometry_changed};

impl Pipeline {
    /// Clear only the captions and the overlay. The OCR preview, gate, persistence
    /// filter, and `last_page` stay, so a layout change mid-page does not restart
    /// the stability wait.
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
            && let Err(e) = o.send(OverlayCommand::Clear)
        {
            warn!(error = %e, "failed to clear overlay captions");
        }
    }

    /// Drop stale captions so they do not stick on the overlay, and forget the
    /// fingerprints and persistence tracks so the same page is translated again if
    /// it comes back. Use this for truly blank screens, not for layout remap misses.
    pub(crate) fn clear_stale_overlay(&mut self, reason: &str) {
        let mut s = self.state.write();
        let had_content = !s.latest_translated_blocks.is_empty() || !s.latest_ocr_blocks.is_empty();

        if !had_content {
            // Still warming up, with icons and flicker filtered out. Leave the status alone.
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

        // Drop the held copies of vanished text, and allow a new translation if it comes back.
        self.persist.reset();
        self.gate.reset_all();
        self.last_translated_fp = None;
        self.last_page = None;
        self.remap_miss_since = None;

        if let Some(o) = self.overlay.as_ref()
            && let Err(e) = o.send(OverlayCommand::Clear)
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

    /// Expire sticky captions once the remap has been empty past the grace period. No new OCR is needed.
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

    /// Run any pending overlay expiry. Call this every capture interval, even when OCR is skipped.
    pub(crate) fn expire_pending(&mut self) {
        self.maybe_expire_raw_empty();
        self.maybe_expire_remap_miss();
    }

    pub(crate) async fn run_ocr_auto(&mut self, frame: &CapturedFrame) {
        let rects = self.ocr_pixel_regions(frame.width, frame.height);

        // Reuse the raw blocks when the OCR scope is unchanged. Only the inference is
        // skipped, and the code below still consumes the blocks every tick.
        let raw = match self.last_raw_ocr.as_mut() {
            Some(cached) if cached.same_content(frame, &rects) => {
                // A new sequence with identical scope pixels, from a target that keeps
                // redrawing. Take the new sequence so stale ticks are skipped at the
                // drive_capture check.
                cached.sequence = frame.sequence;
                debug!(frame = frame.sequence, blocks = cached.blocks.len(), "OCR content cache hit — skipping inference");
                cached.blocks.clone()
            }
            _ => match self.infer(frame, &rects).await {
                Some(raw) => raw,
                None => return,
            },
        };

        self.consume_raw_ocr(raw, frame.width, frame.height).await;
    }

    /// Run OCR on `frame`, record its time and block count, and key the result to this frame.
    ///
    /// Returns `None` when no engine is loaded or inference fails. A failure sets the error on the state.
    async fn infer(&mut self, frame: &CapturedFrame, rects: &[Rect]) -> Option<Vec<OcrBlock>> {
        let ocr_start = Instant::now();
        let engine = self.engine.as_ref()?;
        let blocks = match engine.recognize_rgba_regions(frame.width, frame.height, &frame.rgba, rects).await {
            Ok(b) => b,
            Err(e) => {
                error!(error = %e, "OCR failed");
                self.state.write().set_error(t!("err.ocr", error = e.to_string()));
                return None;
            }
        };
        let ocr_ms = ocr_start.elapsed().as_millis() as u64;
        {
            let mut s = self.state.write();
            s.last_ocr_ms = Some(ocr_ms);
            s.last_ocr_block_count = blocks.len() as u32;
        }
        debug!(ocr_ms, blocks = blocks.len(), frame = frame.sequence, "OCR frame complete");
        self.last_raw_ocr = Some(LastRawOcr {
            sequence: frame.sequence,
            width: frame.width,
            height: frame.height,
            scope: LastRawOcr::pack_scope(frame, rects),
            blocks: blocks.clone(),
        });
        Some(blocks)
    }

    /// Update the status, then run the persistence filter, the stability gate, and the
    /// remap. Runs on every capture tick, including cache hits, where `raw` and the
    /// frame size come from the cache.
    pub(crate) async fn consume_raw_ocr(&mut self, raw: Vec<OcrBlock>, frame_w: u32, frame_h: u32) {
        {
            let mut s = self.state.write();
            if s.translate_in_flight || s.status.is_translating() {
                return;
            }
            // OCR can take hundreds of ms. Keep the waiting and overlay statuses so the UI
            // does not look stuck on "Running OCR" while the text is already stable.
            if !matches!(s.status, PipelineStatus::WaitingForStable { .. } | PipelineStatus::OverlayActive) {
                s.status = PipelineStatus::RunningOcr;
            }
        }

        let content_resized = self.capture_content_resized(frame_w, frame_h);
        if content_resized {
            // Old tracks are in the previous pixel space. Holding them would paint
            // those boxes onto the new content size and freeze the captions.
            self.persist.reset();
        }

        // Persistence keeps vanished text for a short grace period to ride out icons
        // and jitter. That helps the stability gate, but it must not keep ghost text
        // alive for the gate once the screen is really blank. Otherwise captions stick
        // forever when capture stops sending frames on a static empty view.
        let durable = self.persist.filter(raw.clone());

        if raw.is_empty() {
            self.raw_content_since = None;
            self.remap_miss_since = None;
            let grace = self.raw_empty_grace();
            let since = *self.raw_empty_since.get_or_insert_with(Instant::now);
            if durable.is_empty() || since.elapsed() >= grace {
                self.raw_empty_since = None;
                // The screen is blank, so reset everything, including the persistence tracks.
                self.clear_stale_overlay("raw OCR empty");
            }
            // During the grace period, leave the last overlay up, but do not feed the held
            // copies back into the gate as if the text were still on screen.
            return;
        }
        self.raw_empty_since = None;
        let content_since = *self.raw_content_since.get_or_insert_with(Instant::now);
        let max_unstable_ms = self.state.read().config.ocr.max_unstable_ms;
        let content_elapsed = content_since.elapsed();
        let force_unstable = max_unstable_ms > 0 && content_elapsed >= Duration::from_millis(max_unstable_ms);

        // Prefer the durable blocks that passed the persistence filter. If OCR never
        // settles long enough to persist, force the latest raw reading through after
        // max_unstable, so flickering text still gets translated instead of sitting
        // on Capturing forever. After a capture resize the filter was reset, so use
        // the raw blocks right away and let the sticky remap take the new boxes.
        let (blocks, forced_raw) = if content_resized {
            (raw.clone(), false)
        } else if !durable.is_empty() {
            (durable, false)
        } else if force_unstable {
            info!(elapsed_ms = content_elapsed.as_millis() as u64, raw = raw.len(), "OCR thrash — forcing raw blocks past persistence");
            (raw.clone(), true)
        } else {
            // Still waiting for the text to linger or be forced. Keep the OCR preview and
            // do not wipe the overlay every frame, which made flicker look like a failed translate.
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
                // The fingerprint switch is confirmed. Drop the old captions so the wrong
                // text is not painted over a new scene during the stability wait.
                {
                    let mut s = self.state.write();
                    s.latest_ocr_blocks = blocks.clone();
                    s.status = PipelineStatus::WaitingForStable { elapsed_ms: 0 };
                }
                self.clear_translated_captions_only("fingerprint changed");
            }
            StabilityOutcome::Waiting { elapsed_ms } => {
                // A page that was not emitted yet is settling. Keep the sticky captions only
                // when most of them still match, as in partial flicker. Otherwise clear them.
                self.apply_sticky_or_clear_low_hit(&blocks, frame_w, frame_h);
                let mut s = self.state.write();
                s.status = PipelineStatus::WaitingForStable { elapsed_ms };
            }
            StabilityOutcome::Ready { elapsed_ms, fingerprint } => {
                info!(blocks = blocks.len(), elapsed_ms, forced_raw, ?fingerprint, "OCR stable — translating");
                // New content is about to be translated. Do not show the previous page's
                // text over the new scene while the request is in flight.
                {
                    let mut s = self.state.write();
                    s.latest_ocr_blocks = blocks.clone();
                }
                self.clear_translated_captions_only("new page ready for translate");
                // Restart the force-raw clock for the content after this emit.
                self.raw_content_since = Some(Instant::now());
                let page = PendingPage {
                    blocks: blocks.clone(),
                    fingerprint,
                    content_width: frame_w,
                    content_height: frame_h,
                };
                self.last_page = Some(page.clone());
                self.start_translate(page, false);
            }
            StabilityOutcome::AlreadyEmitted { .. } => {
                // The page is held by persistence. Keep the held captions while the
                // durable OCR still matches the last sources. Raw jitter must not hide them.
                self.apply_sticky_overlay(&blocks, frame_w, frame_h);
                if self.state.read().auto_running {
                    let mut s = self.state.write();
                    if !s.latest_translated_blocks.is_empty() {
                        s.status = PipelineStatus::OverlayActive;
                    }
                }
            }
        }
    }

    /// Remap sticky captions when most still match, and clear them on low coverage, which means a page change.
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
        // A low hit rate means most of the content was replaced, so hide the old translations.
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

    /// Remap sticky captions, and clear only the captions when nothing maps for a grace period.
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
                // The durable OCR changed because persistence adopted a new string.
                // Single-frame raw jitter never gets here, because persistence keeps
                // emitting the last source until the new reading lingers.
                self.clear_translated_captions_only("source no longer matches captions");
                return;
            }
            if had_translated {
                let _ = self.remap_miss_since.get_or_insert_with(Instant::now);
                self.maybe_expire_remap_miss();
            }
            // The OCR is empty. Keep the last captions during the short miss grace period.
            return;
        }

        self.remap_miss_since = None;

        // Skip SetBlocks when the geometry is unchanged, which avoids layout flicker.
        // Also skip it when only the capture size changed and the boxes are still in
        // the old pixel space. The overlay host's scaling keeps the captions aligned
        // until the remap takes boxes in the new space (`content_resized` plus new OCR).
        if !geometry_changed {
            return;
        }

        if let Some(o) = self.overlay.as_ref() {
            if let Some(hwnd) = self.state.read().target_hwnd {
                let _ = o.send(OverlayCommand::Attach { target_hwnd: hwnd });
            }
            if let Err(e) = o.send(OverlayCommand::SetBlocks {
                blocks: remapped.clone(),
                content_width: frame_w,
                content_height: frame_h,
            }) {
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
        self.state.write().status = PipelineStatus::RunningOcr;
        let regions = self.ocr_pixel_regions(frame.width, frame.height);
        // Manual capture always infers, and its result keys the frame for auto ticks.
        let Some(blocks) = self.infer(frame, &regions).await else {
            return;
        };
        let fp = OcrFingerprint::from_blocks(&blocks);
        let _ = self.gate.force_emit(fp);

        info!(blocks = blocks.len(), "manual OCR complete — translating");
        let page = PendingPage {
            blocks: blocks.clone(),
            fingerprint: fp,
            content_width: frame.width,
            content_height: frame.height,
        };
        self.last_page = Some(page.clone());
        self.state.write().latest_ocr_blocks = blocks;
        // A manual capture always makes a new API call, since the user asked for it.
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
        if let Some(o) = self.overlay.as_ref() {
            let _ = o.send(OverlayCommand::Clear);
        }
    }
}

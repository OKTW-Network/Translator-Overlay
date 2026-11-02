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
use translator_translate::TranslationCache;

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
            self.change_since = None;
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

        // Prefer durable (settle-filtered) blocks; thrashing text reaches it through
        // the filter's own max_wait force-confirm. After a capture-surface resize,
        // persist was reset — use raw immediately so sticky remap can adopt boxes in
        // the new pixel space.
        let blocks = if content_resized {
            raw
        } else if !durable.is_empty() {
            durable
        } else {
            // Nothing settled: captions whose source left the screen outlived their hold, or
            // persist was just reset (manual capture) and raw still shows their page. Remap
            // onto raw so only the former drop; let a dropped page re-translate if it returns.
            let had_captions = !self.captions.is_empty();
            self.apply_sticky_overlay(&raw, frame_w, frame_h);
            if had_captions && self.captions.is_empty() {
                self.gate.reset_all();
                self.last_page = None;
            }
            let since = *self.change_since.get_or_insert_with(Instant::now);
            if !translating {
                self.state.write().status = PipelineStatus::WaitingForStable {
                    elapsed_ms: since.elapsed().as_millis() as u64,
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
            self.change_since = None;
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
            // Counted from the change, not per unsettled stretch: a thrashing track that
            // persist force-adopts settles for one frame each time and would restart it.
            let since = *self.change_since.get_or_insert_with(Instant::now);
            let (wait_all, max_wait) = {
                let ocr = &self.state.read().config.ocr;
                (ocr.wait_all_settled, ocr.max_wait_ms)
            };
            // max_wait 0 waits as long as it takes.
            if wait_all && !self.persist.settled() && (max_wait == 0 || since.elapsed() < Duration::from_millis(max_wait)) {
                // Text still settling or held over: emitting now would mix old and new lines.
                StabilityOutcome::Waiting {
                    elapsed_ms: since.elapsed().as_millis() as u64,
                }
            } else {
                self.gate.observe(fp)
            }
        };

        match outcome {
            StabilityOutcome::Waiting { elapsed_ms } => {
                // New page still settling: captions whose source is still on screen stay,
                // the rest drop.
                self.apply_sticky_overlay(&blocks, frame_w, frame_h);
                // Settled lines already in the cache show now instead of waiting for the page.
                // A shown line has a caption next frame, so each appearance counts one cache use.
                let cached = {
                    let s = self.state.read();
                    let tcfg = &s.config.translation;
                    let shown: HashSet<String> = self.captions.iter().map(|c| normalize_ocr_text(&c.source)).collect();
                    let hits: Vec<Option<String>> = blocks
                        .iter()
                        .map(|b| {
                            let fresh = tcfg.cache_enabled && !shown.contains(&normalize_ocr_text(&b.text));
                            fresh
                                .then(|| self.translation_cache.lookup(&b.text, &tcfg.source_lang, &tcfg.target_lang))
                                .flatten()
                        })
                        .collect();
                    TranslationCache::stitch(&blocks, &hits, &[])
                };
                if !cached.is_empty() {
                    let mut captions = self.captions.clone();
                    captions.extend(cached);
                    self.paint_captions(captions, frame_w, frame_h);
                }
                self.state.write().status = PipelineStatus::WaitingForStable { elapsed_ms };
            }
            StabilityOutcome::Ready { elapsed_ms, fingerprint } => {
                info!(blocks = blocks.len(), elapsed_ms, ?fingerprint, "OCR stable — translating");
                // Keep captions still on the new page (e.g. shown early from the cache). The
                // translate preview repaints from cache hits only, so without the cache they
                // would blink out at the first streamed row.
                if self.state.read().config.translation.cache_enabled {
                    self.apply_sticky_overlay(&blocks, frame_w, frame_h);
                } else {
                    self.clear_translated_captions_only("new page ready for translate");
                }
                self.change_since = None;
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

    /// Sticky remap onto settled `blocks`; captions whose source left them drop.
    fn apply_sticky_overlay(&mut self, blocks: &[OcrBlock], frame_w: u32, frame_h: u32) {
        let content_resized = self.capture_content_resized(frame_w, frame_h) || self.last_page.is_none();
        let remapped = remap_translations_to_ocr(&self.captions, blocks, content_resized);
        self.set_ocr_blocks(blocks.to_vec());

        if remapped.is_empty() {
            // Durable OCR changed (persist adopted or dropped a string). Raw single-frame
            // jitter must not reach here — persist still emits the last source for a frame.
            self.clear_translated_captions_only("source no longer matches captions");
            return;
        }

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

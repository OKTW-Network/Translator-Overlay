//! Translate job lifecycle (start, finish, apply to overlay / state).

use std::sync::Arc;

use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use translator_core::{PipelineStatus, TranslatedBlock};
use translator_overlay::OverlayController;
use translator_translate::{Completion, TranslateError, TranslationCache, blocks_to_translated_text, merge_translations_detailed};

use crate::pipeline::worker::{InflightTranslate, PendingPage, Pipeline, SharedState};

impl Pipeline {
    pub(crate) fn start_translate(&mut self, page: PendingPage, force: bool) {
        if self.inflight.is_some() {
            return;
        }

        {
            let mut s = self.state.write();
            s.latest_ocr_blocks = page.blocks.clone();
            s.latest_ocr_text = page.source_text.clone();
        }

        if page.blocks.is_empty() || page.source_text.trim().is_empty() {
            self.last_translated_fp = None;
            self.last_page = None;
            let mut s = self.state.write();
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
            if let Some(o) = self.overlay.as_ref()
                && let Err(e) = o.clear()
            {
                warn!(error = %e, "failed to clear overlay on empty page");
            }
            return;
        }

        // Skip identical content unless forced (saves API calls).
        if !force && self.last_translated_fp == Some(page.fingerprint) {
            info!(?page.fingerprint, "skip translate — content unchanged");
            let mut s = self.state.write();
            s.translate_in_flight = false;
            if !s.latest_translated_blocks.is_empty() {
                s.status = PipelineStatus::OverlayActive;
            } else if s.auto_running {
                s.status = PipelineStatus::Capturing;
            }
            return;
        }

        let (api, tcfg) = {
            let s = self.state.read();
            (s.config.api.clone(), s.config.translation.clone())
        };
        self.client.update_api(api);

        let resolved = self.translation_cache.resolve(&page.blocks, &tcfg, force);
        let hit_count = resolved.hits.iter().filter(|h| h.is_some()).count();
        if resolved.misses.is_empty() {
            info!(hits = hit_count, blocks = page.blocks.len(), "translate cache — all hits, skipping API");
            let translated = TranslationCache::stitch(&page.blocks, &resolved.hits, &[]);
            let translated_text = blocks_to_translated_text(&translated);
            self.last_translated_fp = Some(page.fingerprint);
            self.state.write().translation_cache_len = self.translation_cache.len();
            apply_translated(
                &self.state,
                self.overlay.as_ref(),
                page.source_text,
                translated,
                translated_text,
                page.content_width,
                page.content_height,
            );
            return;
        }

        info!(hits = hit_count, unique_misses = resolved.misses.len(), blocks = page.blocks.len(), force, "translate cache partition");

        // Show remembered captions now; new lines wait for the API.
        let preview = TranslationCache::hits_only(&page.blocks, &resolved.hits);
        if !preview.is_empty() {
            apply_cached_preview(&self.state, self.overlay.as_ref(), preview, page.content_width, page.content_height);
        }

        // Canonical conversation prepare (shared with translate_blocks_*).
        let prepared = self.conversation.begin_translate_request(&tcfg, &resolved.misses);

        let cancel = CancellationToken::new();
        let cancel_job = cancel.clone();
        let client_clone = self.client.clone();
        let state_cb = Arc::clone(&self.state);
        let (tx, rx) = oneshot::channel();

        {
            let mut s = self.state.write();
            s.status = PipelineStatus::Translating;
            s.translate_in_flight = true;
            s.last_error = None;
            s.can_retry_translate = true;
        }

        tokio::spawn(async move {
            let result = client_clone
                .complete_with_retry_on(&prepared, &cancel_job, move |attempt, max_retries, error, _backoff_ms| {
                    let message = error.to_string();
                    let mut s = state_cb.write();
                    s.last_error = Some(message.clone());
                    s.status = PipelineStatus::RetryingTranslate {
                        attempt,
                        max_retries,
                        message,
                    };
                })
                .await;
            let _ = tx.send(result);
        });

        self.inflight = Some(InflightTranslate {
            cancel,
            rx,
            fingerprint: page.fingerprint,
            blocks: page.blocks,
            source_text: page.source_text,
            content_width: page.content_width,
            content_height: page.content_height,
            miss_blocks: resolved.misses,
            cached_hits: resolved.hits,
        });
    }

    pub(crate) fn finish_translate(&mut self, job: InflightTranslate, result: Result<Completion, TranslateError>) {
        match result {
            Ok(completion) => match merge_translations_detailed(&job.miss_blocks, &completion.text) {
                Ok(outcome) => {
                    self.conversation.commit_completion(&completion);
                    let tcfg = self.state.read().config.translation.clone();
                    self.translation_cache
                        .store_model_pairs(&job.miss_blocks, &outcome.blocks, &outcome.model_ids, &tcfg);
                    let translated = TranslationCache::stitch(&job.blocks, &job.cached_hits, &outcome.blocks);
                    let translated_text = blocks_to_translated_text(&translated);
                    info!(
                        blocks = translated.len(),
                        cached = job.cached_hits.iter().filter(|h| h.is_some()).count(),
                        turns = self.conversation.turn_count(),
                        "translation complete"
                    );
                    self.last_translated_fp = Some(job.fingerprint);
                    self.state.write().translation_cache_len = self.translation_cache.len();
                    apply_translated(
                        &self.state,
                        self.overlay.as_ref(),
                        job.source_text,
                        translated,
                        translated_text,
                        job.content_width,
                        job.content_height,
                    );
                }
                Err(e) => {
                    error!(error = %e, "failed to merge translation");
                    // Drop the pending user turn so Retry re-sends a clean request
                    // (do not store invalid model JSON as assistant context).
                    self.conversation.rollback_user_turn();
                    let mut s = self.state.write();
                    s.translate_in_flight = false;
                    s.can_retry_translate = true;
                    s.set_error(format!("translate parse: {e}"));
                }
            },
            Err(e) if e.is_cancelled() => {
                info!("translation cancelled");
                self.conversation.rollback_user_turn();
                let mut s = self.state.write();
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
                self.conversation.rollback_user_turn();
                let mut s = self.state.write();
                s.translate_in_flight = false;
                s.can_retry_translate = true;
                s.set_error(format!("translate: {e}"));
            }
        }
    }
}

/// Paint cache hits immediately. Does not finish the job or append history.
fn apply_cached_preview(
    state: &SharedState,
    overlay: Option<&OverlayController>,
    translated: Vec<TranslatedBlock>,
    content_width: u32,
    content_height: u32,
) {
    if let Some(o) = overlay {
        if let Some(hwnd) = state.read().target_hwnd {
            let _ = o.attach(hwnd);
        }
        if let Err(e) = o.set_blocks(translated.clone(), content_width, content_height) {
            warn!(error = %e, "failed to preview cached overlay");
        }
    }

    let translated_text = blocks_to_translated_text(&translated);
    let mut s = state.write();
    s.latest_translated_blocks = translated;
    s.latest_translated_text = translated_text;
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

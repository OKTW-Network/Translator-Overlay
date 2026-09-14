//! Translate job lifecycle (start, finish, apply to overlay / state).

use std::sync::Arc;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use translator_core::{PipelineStatus, TranslatedBlock};
use translator_overlay::{OverlayCommand, OverlayController};
use translator_translate::{
    Completion, TranslateError, TranslationCache, blocks_to_translated_text, merge_translations_detailed, peek_translation_pairs,
};

use crate::{
    attention::flash_control_window_taskbar,
    pipeline::worker::{InflightTranslate, PendingPage, Pipeline, SharedState, TranslateJobMsg},
};

impl Pipeline {
    pub(crate) fn start_translate(&mut self, page: PendingPage, force: bool) {
        if self.inflight.is_some() {
            return;
        }

        {
            let mut s = self.state.write();
            s.latest_ocr_blocks = page.blocks.clone();
        }

        if page.blocks.is_empty() || page.source_text.trim().is_empty() {
            self.last_translated_fp = None;
            self.last_page = None;
            let mut s = self.state.write();
            s.latest_translated_blocks.clear();
            s.translate_in_flight = false;
            if s.auto_running {
                s.status = PipelineStatus::Capturing;
            } else {
                s.status = PipelineStatus::Idle;
            }
            drop(s);
            // Manual / empty-page path must also wipe the live overlay window.
            if let Some(o) = self.overlay.as_ref()
                && let Err(e) = o.send(OverlayCommand::Clear)
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
        self.client.update_config(api);

        let resolved = self.translation_cache.resolve(&page.blocks, &tcfg, force);
        let hit_count = resolved.hits.iter().filter(|h| h.is_some()).count();
        if resolved.misses.is_empty() {
            info!(hits = hit_count, blocks = page.blocks.len(), "translate cache — all hits, skipping API");
            let translated = TranslationCache::stitch(&page.blocks, &resolved.hits, &[]);
            self.last_translated_fp = Some(page.fingerprint);
            self.state.write().translation_cache_len = self.translation_cache.len();
            apply_translated(&self.state, self.overlay.as_ref(), translated, page.content_width, page.content_height);
            return;
        }

        info!(hits = hit_count, unique_misses = resolved.misses.len(), blocks = page.blocks.len(), force, "translate cache partition");

        // Show remembered captions now; new lines wait for the API.
        let preview = TranslationCache::hits_only(&page.blocks, &resolved.hits);
        if !preview.is_empty() {
            apply_cached_preview(&self.state, self.overlay.as_ref(), preview, page.content_width, page.content_height);
        }

        let prepared = self.conversation.begin_translate_request(&tcfg, &resolved.misses);
        self.state.write().history.truncate(self.conversation.turn_count());

        let cancel = CancellationToken::new();
        let cancel_job = cancel.clone();
        let client_clone = self.client.clone();
        let state_cb = Arc::clone(&self.state);
        let (tx, rx) = mpsc::unbounded_channel();
        let tx_done = tx.clone();
        let tx_retry = tx.clone();

        {
            let mut s = self.state.write();
            s.status = PipelineStatus::Translating;
            s.translate_in_flight = true;
            s.last_error = None;
        }

        tokio::spawn(async move {
            let result = client_clone
                .complete_with_retry_on(
                    &prepared,
                    &cancel_job,
                    move |attempt, max_retries, error, _backoff_ms| {
                        let _ = tx_retry.send(TranslateJobMsg::Partial(Vec::new()));
                        let message = error.to_string();
                        let mut s = state_cb.write();
                        s.last_error = Some(message.clone());
                        s.status = PipelineStatus::RetryingTranslate {
                            attempt,
                            max_retries,
                            message,
                        };
                    },
                    &mut move |text| {
                        let pairs = peek_translation_pairs(text);
                        if !pairs.is_empty() {
                            let _ = tx.send(TranslateJobMsg::Partial(pairs));
                        }
                    },
                )
                .await;
            let _ = tx_done.send(TranslateJobMsg::Done(result));
        });

        let revert_blocks = self.state.read().latest_translated_blocks.clone();
        self.inflight = Some(InflightTranslate {
            cancel,
            rx,
            fingerprint: page.fingerprint,
            blocks: page.blocks,
            content_width: page.content_width,
            content_height: page.content_height,
            miss_blocks: resolved.misses,
            cached_hits: resolved.hits,
            revert_blocks,
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
                    info!(
                        blocks = translated.len(),
                        cached = job.cached_hits.iter().filter(|h| h.is_some()).count(),
                        turns = self.conversation.turn_count(),
                        "translation complete"
                    );
                    self.last_translated_fp = Some(job.fingerprint);
                    {
                        let mut s = self.state.write();
                        s.translation_cache_len = self.translation_cache.len();
                        s.push_history(
                            job.blocks.iter().map(|b| b.text.as_str()).collect::<Vec<_>>().join("\n"),
                            blocks_to_translated_text(&translated),
                            tcfg.conversation_max_turns,
                        );
                    }
                    apply_translated(&self.state, self.overlay.as_ref(), translated, job.content_width, job.content_height);
                }
                Err(e) => {
                    error!(error = %e, "failed to merge translation");
                    // Drop the pending user turn so the next translate does not
                    // store invalid model JSON as assistant context.
                    self.conversation.rollback_user_turn();
                    self.apply_stream_preview(&job, &[]);
                    self.fail_translate(format!("translate parse: {e}"));
                }
            },
            Err(e) if e.is_cancelled() => {
                info!("translation cancelled");
                self.conversation.rollback_user_turn();
                self.apply_stream_preview(&job, &[]);
                let mut s = self.state.write();
                s.translate_in_flight = false;
                if s.auto_running {
                    s.status = PipelineStatus::Capturing;
                } else {
                    s.status = PipelineStatus::Cancelled;
                }
            }
            Err(e) => {
                error!(error = %e, "translation failed");
                self.conversation.rollback_user_turn();
                self.apply_stream_preview(&job, &[]);
                self.fail_translate(format!("translate: {e}"));
            }
        }
    }

    fn fail_translate(&self, message: String) {
        let flash = {
            let mut s = self.state.write();
            s.translate_in_flight = false;
            s.set_error(message);
            let flash = !s.attention_sent;
            s.attention_sent = true;
            flash
        };
        if flash {
            flash_control_window_taskbar();
        }
    }

    pub(crate) fn apply_stream_preview(&self, job: &InflightTranslate, pairs: &[(u32, String)]) {
        if pairs.is_empty() && !job.revert_blocks.is_empty() {
            apply_cached_preview(&self.state, self.overlay.as_ref(), job.revert_blocks.clone(), job.content_width, job.content_height);
            return;
        }
        let mut hits = job.cached_hits.clone();
        for (id, text) in pairs {
            if let Some(i) = job.blocks.iter().position(|b| b.id == *id) {
                hits[i] = Some(text.clone());
            }
        }
        let preview = TranslationCache::hits_only(&job.blocks, &hits);
        apply_cached_preview(&self.state, self.overlay.as_ref(), preview, job.content_width, job.content_height);
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
            let _ = o.send(OverlayCommand::Attach { target_hwnd: hwnd });
        }
        if let Err(e) = o.send(OverlayCommand::SetBlocks {
            blocks: translated.clone(),
            content_width,
            content_height,
        }) {
            warn!(error = %e, "failed to preview cached overlay");
        }
    }

    let mut s = state.write();
    s.latest_translated_blocks = translated;
}

fn apply_translated(
    state: &SharedState,
    overlay: Option<&OverlayController>,
    translated: Vec<TranslatedBlock>,
    content_width: u32,
    content_height: u32,
) {
    if let Some(o) = overlay {
        if let Some(hwnd) = state.read().target_hwnd {
            let _ = o.send(OverlayCommand::Attach { target_hwnd: hwnd });
        }
        if let Err(e) = o.send(OverlayCommand::SetBlocks {
            blocks: translated.clone(),
            content_width,
            content_height,
        }) {
            warn!(error = %e, "failed to update overlay");
        }
    }

    let mut s = state.write();
    s.latest_translated_blocks = translated;
    s.translate_in_flight = false;
    s.last_error = None;
    s.status = PipelineStatus::OverlayActive;
    s.attention_sent = false;
}

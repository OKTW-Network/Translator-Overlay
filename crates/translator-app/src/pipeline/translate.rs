//! Translate job lifecycle (start, finish, apply to overlay / state).

use std::sync::Arc;

use rust_i18n::t;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use translator_core::{OcrBlock, PipelineStatus, TranslatedBlock};
use translator_ocr::OcrEngine;
use translator_overlay::OverlayCommand;
use translator_translate::{
    Completion, TranslateError, TranslationCache, blocks_to_translated_text, merge_translations, peek_translation_pairs, should_retry,
};

use crate::{
    attention::flash_control_window_taskbar,
    pipeline::{
        ping_ui,
        worker::{InflightTranslate, PendingPage, Pipeline, TranslateJobMsg, remap_translations_to_ocr},
    },
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

        if page.blocks.iter().all(|b| b.text.trim().is_empty()) {
            self.last_translated_fp = None;
            self.last_page = None;
            {
                let mut s = self.state.write();
                s.latest_translated_blocks.clear();
                s.translate_in_flight = false;
                s.restore_operational_status();
            }
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
            s.restore_operational_status();
            return;
        }

        let tcfg = self.state.read().config.translation.clone();
        let resolved = self.translation_cache.resolve(&page.blocks, &tcfg, force);
        let hit_count = resolved.hits.iter().filter(|h| h.is_some()).count();
        if resolved.misses.is_empty() {
            info!(hits = hit_count, blocks = page.blocks.len(), "translate cache — all hits, skipping API");
            let translated = TranslationCache::stitch(&page.blocks, &resolved.hits, &[]);
            self.last_translated_fp = Some(page.fingerprint);
            self.state.write().translation_cache_len = self.translation_cache.len();
            self.apply_translated(translated, page.content_width, page.content_height);
            return;
        }

        info!(hits = hit_count, unique_misses = resolved.misses.len(), blocks = page.blocks.len(), force, "translate cache partition");
        self.launch_translate_job(page, resolved.hits, resolved.misses, 0);
    }

    /// Load the built-in translation model in the background so the first page does not wait
    /// for it. A translation started meanwhile queues behind the load.
    pub(crate) fn start_preload(&mut self) {
        self.cancel_preload();
        let cancel = CancellationToken::new();
        self.preload = Some(cancel.clone());
        let client = self.client.clone();
        let state = Arc::clone(&self.state);
        tokio::spawn(async move {
            let on_status = &mut |load| {
                state.write().translation_model_load = load;
                ping_ui();
            };
            match client.preload(&cancel, on_status).await {
                Err(e) if !e.is_cancelled() => {
                    error!(error = %e, "translation model preload failed");
                    state.write().set_error(t!("err.translate", error = e.to_string()));
                    ping_ui();
                }
                _ => {}
            }
        });
    }

    pub(crate) fn cancel_preload(&mut self) {
        if let Some(cancel) = self.preload.take() {
            cancel.cancel();
        }
    }

    fn launch_translate_job(
        &mut self,
        page: PendingPage,
        cached_hits: Vec<Option<String>>,
        miss_blocks: Vec<OcrBlock>,
        start_attempt: u32,
    ) {
        let (api, tcfg, glossary) = {
            let s = self.state.read();
            (s.config.api.clone(), s.config.translation.clone(), s.dictionary_pairs.clone())
        };
        let max_retries = api.max_retries;
        self.client.update_config(api);

        // Show remembered captions now; new lines wait for the API.
        let preview = TranslationCache::stitch(&page.blocks, &cached_hits, &[]);
        if !preview.is_empty() {
            self.show_captions(preview, page.content_width, page.content_height);
        }

        let prepared = self.conversation.begin_translate_request(&tcfg, &miss_blocks, &glossary);
        self.state.write().history.truncate(self.conversation.turn_count());

        let cancel = CancellationToken::new();
        let cancel_job = cancel.clone();
        let client_clone = self.client.clone();
        let state_cb = Arc::clone(&self.state);
        let state_status = Arc::clone(&self.state);
        let (tx, rx) = mpsc::unbounded_channel();
        let tx_done = tx.clone();
        let tx_retry = tx.clone();

        {
            let mut s = self.state.write();
            s.translate_in_flight = true;
            if start_attempt == 0 {
                s.status = PipelineStatus::Translating;
                s.last_error = None;
            } else {
                s.status = PipelineStatus::RetryingTranslate {
                    attempt: start_attempt,
                    max_retries,
                    message: s.last_error.clone().unwrap_or_default(),
                };
            }
        }

        tokio::spawn(async move {
            let result = client_clone
                .complete_with_retry_on(
                    &prepared,
                    &cancel_job,
                    start_attempt,
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
                    &mut move |load| {
                        state_status.write().translation_model_load = load;
                        ping_ui();
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
            miss_blocks,
            cached_hits,
            revert_blocks,
        });
    }

    pub(crate) fn finish_translate(&mut self, job: InflightTranslate, result: Result<(Completion, u32), TranslateError>) {
        match result {
            Ok((completion, retry_attempt)) => match merge_translations(&job.miss_blocks, &completion.text) {
                Ok(blocks) => {
                    self.conversation.commit_completion(&completion);
                    let (tcfg, max_retries) = {
                        let s = self.state.read();
                        (s.config.translation.clone(), s.config.api.max_retries)
                    };
                    self.translation_cache.store_model_pairs(&blocks, &tcfg);
                    let translated = TranslationCache::stitch(&job.blocks, &job.cached_hits, &blocks);
                    let miss_blocks: Vec<OcrBlock> = job
                        .miss_blocks
                        .into_iter()
                        .filter(|b| !blocks.iter().any(|t| t.id == b.id))
                        .collect();
                    if !miss_blocks.is_empty() {
                        let err = TranslateError::MissingBlocks(miss_blocks.iter().map(|b| b.id).collect());
                        let hits = should_retry(&err, retry_attempt, max_retries).then(|| {
                            let mut hits = vec![None; job.blocks.len()];
                            for tb in &translated {
                                if let Some(i) = job.blocks.iter().position(|b| b.id == tb.id) {
                                    hits[i] = Some(tb.translation.clone());
                                }
                            }
                            hits
                        });
                        self.show_captions(translated, job.content_width, job.content_height);
                        self.state.write().translation_cache_len = self.translation_cache.len();
                        if let Some(hits) = hits {
                            info!(
                                attempt = retry_attempt + 1,
                                max_retries,
                                error = %err,
                                "response omitted blocks; resending"
                            );
                            self.state.write().last_error = Some(err.to_string());
                            let page = PendingPage {
                                blocks: job.blocks,
                                fingerprint: job.fingerprint,
                                content_width: job.content_width,
                                content_height: job.content_height,
                            };
                            self.launch_translate_job(page, hits, miss_blocks, retry_attempt + 1);
                            return;
                        }
                        error!(error = %err, "response omitted blocks; retries exhausted");
                        self.fail_translate(t!("err.translate", error = err.to_string()).into_owned());
                        return;
                    }
                    info!(
                        blocks = translated.len(),
                        cached = job.cached_hits.iter().filter(|h| h.is_some()).count(),
                        turns = self.conversation.turn_count(),
                        "translation complete"
                    );
                    {
                        let mut s = self.state.write();
                        s.translation_cache_len = self.translation_cache.len();
                        s.push_history(
                            OcrEngine::blocks_to_text(&job.blocks),
                            blocks_to_translated_text(&translated),
                            tcfg.conversation_max_turns,
                        );
                    }
                    self.apply_translated(translated, job.content_width, job.content_height);
                    if self.state.read().latest_translated_blocks.is_empty() {
                        // The page left the screen mid-translation: let it emit again (from the cache) on return.
                        self.last_page = None;
                        self.gate.reset_all();
                    } else {
                        self.last_translated_fp = Some(job.fingerprint);
                    }
                }
                Err(e) => {
                    // Drop the pending user turn so the next translate does not
                    // store invalid model JSON as assistant context.
                    self.conversation.rollback_user_turn();
                    self.apply_stream_preview(&job, &[]);
                    let max_retries = self.state.read().config.api.max_retries;
                    // A cut-off or malformed reply (e.g. a dropped closing quote) often parses on the next try.
                    if retry_attempt < max_retries {
                        warn!(attempt = retry_attempt + 1, max_retries, error = %e, "unparsable translation; resending");
                        self.state.write().last_error = Some(e.to_string());
                        let page = PendingPage {
                            blocks: job.blocks,
                            fingerprint: job.fingerprint,
                            content_width: job.content_width,
                            content_height: job.content_height,
                        };
                        self.launch_translate_job(page, job.cached_hits, job.miss_blocks, retry_attempt + 1);
                        return;
                    }
                    error!(error = %e, "failed to merge translation");
                    self.fail_translate(t!("err.translate_parse", error = e.to_string()).into_owned());
                }
            },
            // Turn-based compression never fires while every request fails, so trim on overflow.
            // Once trimmed, another overflow falls through to the plain error.
            Err(TranslateError::ContextFull(context))
                if self.conversation.turn_count() > self.state.read().config.translation.history_max_items =>
            {
                self.conversation.rollback_user_turn();
                let turns = self.conversation.turn_count();
                let keep = self.state.read().config.translation.history_max_items;
                self.conversation.compress_if_needed(0, keep);
                info!(context, turns, keep, "context full; trimmed history and resending");
                // Restore the captions first: the resent job keeps them as its revert point.
                self.apply_stream_preview(&job, &[]);
                let page = PendingPage {
                    blocks: job.blocks,
                    fingerprint: job.fingerprint,
                    content_width: job.content_width,
                    content_height: job.content_height,
                };
                self.launch_translate_job(page, job.cached_hits, job.miss_blocks, 0);
            }
            Err(e) if e.is_cancelled() => {
                info!("translation cancelled");
                self.conversation.rollback_user_turn();
                self.apply_stream_preview(&job, &[]);
                let mut s = self.state.write();
                s.translate_in_flight = false;
                if s.capture_paused {
                    s.status = PipelineStatus::Paused;
                } else if s.auto_running {
                    s.status = PipelineStatus::Capturing;
                } else {
                    s.status = PipelineStatus::Cancelled;
                }
            }
            Err(e) => {
                error!(error = %e, "translation failed");
                self.conversation.rollback_user_turn();
                self.apply_stream_preview(&job, &[]);
                self.fail_translate(t!("err.translate", error = e.to_string()).into_owned());
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
            self.show_captions(job.revert_blocks.clone(), job.content_width, job.content_height);
            return;
        }
        let mut hits = job.cached_hits.clone();
        for (id, text) in pairs {
            if let Some(i) = job.blocks.iter().position(|b| b.id == *id) {
                hits[i] = Some(text.clone());
            }
        }
        let preview = TranslationCache::stitch(&job.blocks, &hits, &[]);
        self.show_captions(preview, job.content_width, job.content_height);
    }

    /// Send captions to the overlay and store them on the state. Does not finish the job or
    /// append history, so it also paints cache hits and stream previews.
    fn show_captions(&self, translated: Vec<TranslatedBlock>, content_width: u32, content_height: u32) {
        // Map onto the current OCR: the page may have changed or the window resized while translating.
        let (w, h) = self
            .last_raw_ocr
            .as_ref()
            .map_or((content_width, content_height), |c| (c.width, c.height));
        let resized = (w, h) != (content_width, content_height);
        let translated = remap_translations_to_ocr(&translated, &self.state.read().latest_ocr_blocks, resized);
        if let Some(o) = self.overlay.as_ref() {
            if let Some(hwnd) = self.state.read().target_hwnd {
                let _ = o.send(OverlayCommand::Attach { target_hwnd: hwnd });
            }
            if let Err(e) = o.send(OverlayCommand::SetBlocks {
                blocks: translated.clone(),
                locale: self.state.read().config.translation.target_lang.clone(),
                content_width: w,
                content_height: h,
            }) {
                warn!(error = %e, "failed to update overlay captions");
            }
        }
        self.state.write().latest_translated_blocks = translated;
    }

    /// Show a finished page and mark the translation done.
    fn apply_translated(&self, translated: Vec<TranslatedBlock>, content_width: u32, content_height: u32) {
        self.show_captions(translated, content_width, content_height);
        let mut s = self.state.write();
        s.translate_in_flight = false;
        s.last_error = None;
        // Not OverlayActive when nothing was kept (the page left the screen).
        s.restore_operational_status();
        s.attention_sent = false;
    }
}

#[cfg(test)]
mod tests {
    use parking_lot::RwLock;
    use translator_capture::CaptureSession;
    use translator_core::{AppConfig, AppState, ModelProvider, Rect};
    use translator_ocr::{BlockPersistenceFilter, OcrFingerprint, StabilityGate};
    use translator_translate::{Conversation, TranslateClient};

    use super::*;
    use crate::pipeline::worker::LastRawOcr;

    fn block(id: u32, text: &str) -> OcrBlock {
        OcrBlock {
            id,
            text: text.into(),
            confidence: 1.0,
            bbox: Rect::new(0.0, 0.0, 10.0, 10.0),
            source_lines: 1,
            source_height: 10.0,
            source_width: 10.0,
        }
    }

    fn pipeline(max_retries: u32) -> Pipeline {
        let mut config = AppConfig::default();
        config.api.max_retries = max_retries;
        // Not the built-in default: a spawned job must never download or run a model.
        config.api.provider = ModelProvider::OpenaiCompatible;
        Pipeline {
            session: CaptureSession::new(),
            gate: StabilityGate::from_config(&config.ocr),
            persist: BlockPersistenceFilter::from_config(&config.ocr),
            engine: None,
            conversation: Conversation::empty(),
            translation_cache: TranslationCache::new(config.translation.cache_max_entries_clamped()),
            client: TranslateClient::new(config.api.clone()),
            last_translated_fp: None,
            last_page: None,
            inflight: None,
            preload: None,
            model_load: None,
            ocr_tier: config.ocr.model_tier,
            ocr_device: config.ocr.device,
            ocr_layout: config.ocr.uses_layout(),
            last_raw_ocr: None,
            preview_seq_bias: 0,
            raw_empty_since: None,
            raw_content_since: None,
            remap_miss_since: None,
            overlay: None,
            state: Arc::new(RwLock::new(AppState::new(config))),
        }
    }

    fn reply(text: &str, attempt: u32) -> Result<(Completion, u32), TranslateError> {
        Ok((
            Completion {
                text: text.into(),
                replay_items: Vec::new(),
            },
            attempt,
        ))
    }

    #[tokio::test]
    async fn omitted_block_is_resent_alone_then_fails_when_retries_run_out() {
        let mut p = pipeline(1);
        let blocks = vec![block(1, "A"), block(2, "B")];
        p.start_translate(
            PendingPage {
                fingerprint: OcrFingerprint::from_blocks(&blocks),
                blocks,
                content_width: 100,
                content_height: 100,
            },
            false,
        );

        // First reply omits block 2: keep block 1, resend only block 2 as attempt 1.
        let job = p.inflight.take().expect("first job");
        p.finish_translate(job, reply(r#"{"b":[[1,"T1"]]}"#, 0));
        let retry = p.inflight.as_ref().expect("retry job for the omitted block");
        assert_eq!(retry.miss_blocks.iter().map(|b| b.id).collect::<Vec<_>>(), [2]);
        assert_eq!(retry.cached_hits, [Some("T1".to_string()), None]);
        assert_eq!(p.state.read().latest_translated_blocks.len(), 1);
        assert!(matches!(p.state.read().status, PipelineStatus::RetryingTranslate { attempt: 1, .. }));

        // Retry omits block 2 again with the budget spent: fail instead of looping.
        let job = p.inflight.take().expect("retry job");
        p.finish_translate(job, reply(r#"{"b":[]}"#, 1));
        assert!(p.inflight.is_none(), "exhausted retries must not launch another job");
        let s = p.state.read();
        assert!(matches!(s.status, PipelineStatus::Error { .. }), "{:?}", s.status);
        assert!(!s.translate_in_flight);
        assert_eq!(p.translation_cache.len(), 1, "the translated block stays cached");
    }

    #[tokio::test]
    async fn unparsable_reply_is_resent_then_fails_when_retries_run_out() {
        let mut p = pipeline(1);
        let blocks = vec![block(1, "A")];
        p.start_translate(
            PendingPage {
                fingerprint: OcrFingerprint::from_blocks(&blocks),
                blocks,
                content_width: 100,
                content_height: 100,
            },
            false,
        );

        // Dropped closing quote: resend the same block as attempt 1 without keeping the bad turn.
        let job = p.inflight.take().expect("first job");
        p.finish_translate(job, reply(r#"{"b":[[1,"T]}"#, 0));
        let retry = p.inflight.as_ref().expect("retry job for the unparsable reply");
        assert_eq!(retry.miss_blocks.iter().map(|b| b.id).collect::<Vec<_>>(), [1]);
        assert!(matches!(p.state.read().status, PipelineStatus::RetryingTranslate { attempt: 1, .. }));

        let job = p.inflight.take().expect("retry job");
        p.finish_translate(job, reply(r#"{"b":[[1,"T]}"#, 1));
        assert!(p.inflight.is_none(), "exhausted retries must not launch another job");
        assert!(matches!(p.state.read().status, PipelineStatus::Error { .. }));
        assert_eq!(p.conversation.turn_count(), 0, "no unparsable turn is kept");
    }

    #[tokio::test]
    async fn context_full_trims_history_once_then_fails() {
        let mut p = pipeline(0);
        p.state.write().config.translation.history_max_items = 1;
        let page = |text: &str| {
            let blocks = vec![block(1, text)];
            PendingPage {
                fingerprint: OcrFingerprint::from_blocks(&blocks),
                blocks,
                content_width: 100,
                content_height: 100,
            }
        };
        for text in ["A", "B", "C"] {
            p.start_translate(page(text), false);
            let job = p.inflight.take().expect("job");
            p.finish_translate(job, reply(r#"{"b":[[1,"T"]]}"#, 0));
        }
        assert_eq!(p.conversation.turn_count(), 3);

        p.start_translate(page("D"), false);
        let job = p.inflight.take().expect("job");
        p.finish_translate(job, Err(TranslateError::ContextFull(4096)));
        assert!(p.inflight.is_some(), "resent with a trimmed history");
        assert_eq!(p.conversation.turn_count(), 1);

        let job = p.inflight.take().expect("job");
        p.finish_translate(job, Err(TranslateError::ContextFull(4096)));
        assert!(p.inflight.is_none(), "a trimmed history that still overflows must fail");
        assert!(matches!(p.state.read().status, PipelineStatus::Error { .. }));
    }

    #[tokio::test]
    async fn vanished_line_is_captioned_again_on_return() {
        let mut p = pipeline(0);
        // Settle off: durable OCR is the raw frame, so no sleeps.
        p.persist = BlockPersistenceFilter::new(0, 750, 0);
        let both = vec![block(1, "X"), block(2, "Y")];
        p.consume_raw_ocr(both.clone(), 100, 100).await;
        p.consume_raw_ocr(both.clone(), 100, 100).await;
        let job = p.inflight.take().expect("page translates");
        p.finish_translate(job, reply(r#"{"b":[[1,"TX"],[2,"TY"]]}"#, 0));
        assert_eq!(p.state.read().latest_translated_blocks.len(), 2);

        p.consume_raw_ocr(vec![block(1, "X")], 100, 100).await;
        assert_eq!(p.state.read().latest_translated_blocks.len(), 1, "vanished line drops its caption");

        p.consume_raw_ocr(both.clone(), 100, 100).await;
        p.consume_raw_ocr(both, 100, 100).await;
        assert_eq!(p.state.read().latest_translated_blocks.len(), 2, "returning line is captioned again");
    }

    #[tokio::test]
    async fn result_only_paints_lines_still_on_screen() {
        let mut p = pipeline(0);
        p.persist = BlockPersistenceFilter::new(0, 750, 0);
        let page = vec![block(1, "X"), block(2, "Y")];
        p.consume_raw_ocr(page.clone(), 100, 100).await;
        p.consume_raw_ocr(page, 100, 100).await;
        assert!(p.inflight.is_some(), "page translates");

        // Y is replaced while waiting: its caption must not land on the new text.
        let changed = vec![block(1, "X"), block(2, "Z")];
        p.consume_raw_ocr(changed.clone(), 100, 100).await;
        p.consume_raw_ocr(changed.clone(), 100, 100).await;
        let job = p.inflight.take().expect("the request is not cancelled");
        p.finish_translate(job, reply(r#"{"b":[[1,"TX"],[2,"TY"]]}"#, 0));
        assert_eq!(
            p.state
                .read()
                .latest_translated_blocks
                .iter()
                .map(|b| b.translation.as_str())
                .collect::<Vec<_>>(),
            ["TX"]
        );
        assert_eq!(p.translation_cache.len(), 2, "the hidden line is still cached");

        // The gate saw nothing while busy, so the changed page still translates.
        p.consume_raw_ocr(changed.clone(), 100, 100).await;
        p.consume_raw_ocr(changed, 100, 100).await;
        let job = p.inflight.as_ref().expect("changed page translates");
        assert_eq!(job.miss_blocks.iter().map(|b| b.text.as_str()).collect::<Vec<_>>(), ["Z"]);
    }

    #[tokio::test]
    async fn page_gone_during_translation_shows_from_cache_on_return() {
        let mut p = pipeline(0);
        p.persist = BlockPersistenceFilter::new(0, 750, 0);
        let x = vec![block(1, "X")];
        p.consume_raw_ocr(x.clone(), 100, 100).await;
        p.consume_raw_ocr(x.clone(), 100, 100).await;
        let b = vec![block(1, "B")];
        p.consume_raw_ocr(b.clone(), 100, 100).await;
        p.consume_raw_ocr(b.clone(), 100, 100).await;
        let job = p.inflight.take().expect("job");
        p.finish_translate(job, reply(r#"{"b":[[1,"TX"]]}"#, 0));
        assert!(p.state.read().latest_translated_blocks.is_empty(), "nothing painted over new text");
        assert_ne!(p.state.read().status, PipelineStatus::OverlayActive, "no captions, no overlay status");

        // The gate saw nothing while busy, so the new page still translates.
        p.consume_raw_ocr(b.clone(), 100, 100).await;
        p.consume_raw_ocr(b, 100, 100).await;
        let job = p.inflight.take().expect("new page translates");
        p.finish_translate(job, reply(r#"{"b":[[1,"TB"]]}"#, 0));

        p.consume_raw_ocr(x.clone(), 100, 100).await;
        p.consume_raw_ocr(x, 100, 100).await;
        assert_eq!(p.state.read().latest_translated_blocks[0].translation, "TX");
    }

    #[tokio::test]
    async fn blank_screen_clears_pending_captions_after_one_hold() {
        let mut p = pipeline(0);
        p.state.write().config.ocr.hold_ms = 20;
        p.persist = BlockPersistenceFilter::new(20, 20, 0);
        p.gate = StabilityGate::new(0);
        let settle = || std::thread::sleep(std::time::Duration::from_millis(30));
        let x = vec![block(1, "X")];
        p.consume_raw_ocr(x.clone(), 100, 100).await;
        settle();
        p.consume_raw_ocr(x.clone(), 100, 100).await;
        p.consume_raw_ocr(x, 100, 100).await;
        let job = p.inflight.as_ref().expect("page translates");
        // The settle filter renumbers block ids from 0.
        p.apply_stream_preview(job, &[(0, "TX".into())]);
        assert_eq!(p.state.read().latest_translated_blocks.len(), 1);

        p.consume_raw_ocr(Vec::new(), 100, 100).await;
        assert_eq!(p.state.read().latest_translated_blocks.len(), 1, "held for hold_ms");
        settle();
        p.consume_raw_ocr(Vec::new(), 100, 100).await;
        assert!(p.state.read().latest_translated_blocks.is_empty(), "cleared once the hold elapses");
        assert_eq!(p.state.read().status, PipelineStatus::Translating, "the blank reset keeps the translating status");
    }

    #[tokio::test]
    async fn result_lands_in_the_resized_pixel_space() {
        let mut p = pipeline(0);
        p.persist = BlockPersistenceFilter::new(0, 750, 0);
        let x = vec![block(1, "X")];
        p.consume_raw_ocr(x.clone(), 100, 100).await;
        p.consume_raw_ocr(x, 100, 100).await;
        assert!(p.inflight.is_some(), "page translates");

        // The window doubles in size before the reply.
        let big = OcrBlock {
            bbox: Rect::new(0.0, 0.0, 20.0, 20.0),
            ..block(1, "X")
        };
        p.last_raw_ocr = Some(LastRawOcr {
            sequence: 0,
            width: 200,
            height: 200,
            scope: Vec::new(),
            blocks: Vec::new(),
        });
        p.consume_raw_ocr(vec![big.clone()], 200, 200).await;
        let job = p.inflight.take().expect("job");
        p.finish_translate(job, reply(r#"{"b":[[1,"TX"]]}"#, 0));
        assert_eq!(p.state.read().latest_translated_blocks[0].bbox, big.bbox);
    }

    #[tokio::test]
    async fn unsettled_ocr_during_translation_keeps_page_paintable() {
        let mut p = pipeline(0);
        p.persist = BlockPersistenceFilter::new(20, 20, 0);
        p.gate = StabilityGate::new(0);
        let x = vec![block(1, "X")];
        p.consume_raw_ocr(x.clone(), 100, 100).await;
        std::thread::sleep(std::time::Duration::from_millis(30));
        p.consume_raw_ocr(x.clone(), 100, 100).await;
        p.consume_raw_ocr(x.clone(), 100, 100).await;
        assert!(p.inflight.is_some(), "page translates");

        // A manual capture resets persist: nothing is settled while the request runs.
        p.persist.reset();
        p.consume_raw_ocr(x, 100, 100).await;
        let job = p.inflight.as_ref().expect("job");
        // The settle filter renumbers block ids from 0.
        p.apply_stream_preview(job, &[(0, "TX".into())]);
        assert_eq!(p.state.read().latest_translated_blocks.len(), 1, "stream preview still paints");
    }

    #[tokio::test]
    async fn page_returning_after_hold_expiry_is_translated_again() {
        let mut p = pipeline(0);
        p.state.write().config.ocr.hold_ms = 20;
        p.persist = BlockPersistenceFilter::new(20, 20, 0);
        p.gate = StabilityGate::new(0);
        let settle = || std::thread::sleep(std::time::Duration::from_millis(30));
        let x = vec![block(1, "X")];
        p.consume_raw_ocr(x.clone(), 100, 100).await;
        settle();
        p.consume_raw_ocr(x.clone(), 100, 100).await;
        p.consume_raw_ocr(x.clone(), 100, 100).await;
        let job = p.inflight.take().expect("page translates");
        // The settle filter renumbers block ids from 0.
        p.finish_translate(job, reply(r#"{"b":[[0,"TX"]]}"#, 0));
        assert_eq!(p.state.read().latest_translated_blocks.len(), 1);

        // Dialog closes over flickering text that never settles: captions expire after hold.
        let noise = |text: &str| {
            vec![OcrBlock {
                bbox: Rect::new(60.0, 60.0, 10.0, 10.0),
                ..block(1, text)
            }]
        };
        settle();
        p.consume_raw_ocr(noise("n1"), 100, 100).await;
        settle();
        p.consume_raw_ocr(noise("n2"), 100, 100).await;
        assert!(p.state.read().latest_translated_blocks.is_empty());

        p.consume_raw_ocr(x.clone(), 100, 100).await;
        settle();
        p.consume_raw_ocr(x.clone(), 100, 100).await;
        p.consume_raw_ocr(x, 100, 100).await;
        assert_eq!(p.state.read().latest_translated_blocks.len(), 1, "same page is translated again");
    }
}

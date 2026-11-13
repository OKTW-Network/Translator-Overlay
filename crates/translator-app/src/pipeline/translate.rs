//! Translate job lifecycle (start, finish, apply to overlay / state).

use std::sync::Arc;

use rust_i18n::t;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use translator_core::{OcrBlock, PipelineStatus, TranslatedBlock};
use translator_ocr::OcrEngine;
use translator_overlay::OverlayCommand;
use translator_translate::{
    Conversation, PageResult, TranslateError, TranslationCache, blocks_to_translated_text, peek_translation_pairs, translate_page,
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
        self.set_ocr_blocks(page.blocks.clone());

        if page.blocks.iter().all(|b| b.text.trim().is_empty()) {
            self.last_page = None;
            self.set_captions(Vec::new());
            self.state.write().restore_operational_status();
            // Manual / empty-page path must also wipe the live overlay window.
            self.send_overlay(OverlayCommand::Clear);
            return;
        }

        let tcfg = self.state.read().config.translation.clone();
        let resolved = self.translation_cache.resolve(&page.blocks, &tcfg, force);
        let hit_count = resolved.hits.iter().filter(|h| h.is_some()).count();
        if resolved.misses.is_empty() {
            info!(hits = hit_count, blocks = page.blocks.len(), "translate cache — all hits, skipping API");
            let translated = TranslationCache::stitch(&page.blocks, &resolved.hits, &[]);
            self.state.write().translation_cache_len = self.translation_cache.len();
            self.apply_translated(translated, page.content_width, page.content_height);
            return;
        }

        info!(hits = hit_count, unique_misses = resolved.misses.len(), blocks = page.blocks.len(), force, "translate cache partition");
        self.launch_translate_job(page, resolved.hits, resolved.misses);
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

    /// Translate `miss_blocks` on a copy of the conversation; progress arrives as
    /// [`TranslateJobMsg`]s and [`Self::finish_translate`] adopts the result.
    fn launch_translate_job(&mut self, page: PendingPage, cached_hits: Vec<Option<String>>, miss_blocks: Vec<OcrBlock>) {
        let (tcfg, glossary, max_retries) = {
            let s = self.state.read();
            (s.config.translation.clone(), s.dictionary_pairs.clone(), s.config.api.max_retries)
        };
        let cancel = CancellationToken::new();
        let (tx, rx) = mpsc::unbounded_channel();
        let job = InflightTranslate {
            cancel: cancel.clone(),
            rx,
            page,
            cached_hits,
            kept: Vec::new(),
        };

        // Show remembered captions now; new lines wait for the API.
        let preview = job.preview(&[]);
        if !preview.is_empty() {
            self.show_captions(preview, job.page.content_width, job.page.content_height);
        }
        {
            let mut s = self.state.write();
            s.status = PipelineStatus::Translating;
            s.last_error = None;
        }

        let conversation = self.conversation.clone();
        let client = self.client.clone();
        let state = Arc::clone(&self.state);
        tokio::spawn(async move {
            let complete = |request: Conversation, attempt: u32| {
                let (client, cancel, state) = (client.clone(), cancel.clone(), Arc::clone(&state));
                let (tx_retry, tx_text) = (tx.clone(), tx.clone());
                async move {
                    client
                        .complete_with_retry_on(
                            &request,
                            &cancel,
                            attempt,
                            move |attempt, max_retries, error, _backoff_ms| {
                                let _ = tx_retry.send(TranslateJobMsg::Retrying {
                                    attempt,
                                    max_retries,
                                    message: error.to_string(),
                                    kept: None,
                                });
                            },
                            &mut move |load| {
                                state.write().translation_model_load = load;
                                ping_ui();
                            },
                            &mut move |text| {
                                let pairs = peek_translation_pairs(text);
                                if !pairs.is_empty() {
                                    let _ = tx_text.send(TranslateJobMsg::Partial(pairs));
                                }
                            },
                        )
                        .await
                }
            };
            let result =
                translate_page(conversation, &tcfg, &glossary, miss_blocks, max_retries, complete, |attempt, error, translated| {
                    let _ = tx.send(TranslateJobMsg::Retrying {
                        attempt,
                        max_retries,
                        message: error.to_string(),
                        kept: Some(translated.to_vec()),
                    });
                })
                .await;
            let _ = tx.send(TranslateJobMsg::Done(result));
        });

        self.set_inflight(job);
    }

    pub(crate) fn on_translate_msg(&mut self, msg: Option<TranslateJobMsg>) {
        match msg {
            Some(TranslateJobMsg::Partial(pairs)) => self.apply_stream_preview(&pairs),
            Some(TranslateJobMsg::Retrying {
                attempt,
                max_retries,
                message,
                kept,
            }) => {
                if let (Some(job), Some(kept)) = (self.inflight.as_mut(), kept) {
                    job.kept = kept;
                }
                // Drop the half-streamed reply; keep what earlier passes translated.
                self.apply_stream_preview(&[]);
                let mut s = self.state.write();
                s.last_error = Some(message.clone());
                s.status = PipelineStatus::RetryingTranslate {
                    attempt,
                    max_retries,
                    message,
                };
            }
            Some(TranslateJobMsg::Done(result)) => {
                let job = self.take_inflight().expect("inflight present");
                self.finish_translate(job, result);
            }
            None => {
                let job = self.take_inflight().expect("inflight present");
                let result = PageResult {
                    conversation: self.conversation.clone(),
                    translated: Vec::new(),
                    error: Some(TranslateError::Other("translate task dropped".into())),
                };
                self.finish_translate(job, result);
            }
        }
    }

    pub(crate) fn finish_translate(&mut self, job: InflightTranslate, result: PageResult) {
        // A Reset or a new capture started a new session meanwhile; keep its empty history.
        let adopted = result.conversation.session_id == self.conversation.session_id;
        if adopted {
            self.conversation = result.conversation;
        }
        let tcfg = self.state.read().config.translation.clone();
        // Partial results are cached too: a resend only asks for what is still missing.
        self.translation_cache.store_model_pairs(&result.translated, &tcfg);
        self.state.write().translation_cache_len = self.translation_cache.len();
        let translated = TranslationCache::stitch(&job.page.blocks, &job.cached_hits, &result.translated);
        let (w, h) = (job.page.content_width, job.page.content_height);

        match result.error {
            None => {
                info!(
                    blocks = translated.len(),
                    cached = job.cached_hits.iter().filter(|h| h.is_some()).count(),
                    turns = self.conversation.turn_count(),
                    "translation complete"
                );
                if adopted {
                    self.state.write().push_history(
                        OcrEngine::blocks_to_text(&job.page.blocks),
                        blocks_to_translated_text(&translated),
                        tcfg.conversation_max_turns,
                    );
                }
                self.apply_translated(translated, w, h);
                if self.captions.is_empty() {
                    // The page left the screen mid-translation: let it emit again (from the cache) on return.
                    self.last_page = None;
                    self.gate.reset_all();
                }
            }
            Some(e) if e.is_cancelled() => {
                info!("translation cancelled");
                self.show_captions(translated, w, h);
                self.state.write().restore_after_cancel();
            }
            Some(e) => {
                error!(error = %e, "translation failed");
                self.show_captions(translated, w, h);
                let message = match e {
                    TranslateError::Parse(_) => t!("err.translate_parse", error = e.to_string()),
                    _ => t!("err.translate", error = e.to_string()),
                };
                self.fail_translate(message.into_owned());
            }
        }
        if adopted {
            // The history rows follow the model context when it is trimmed (after this page's row).
            self.state.write().history.truncate(self.conversation.turn_count());
        }
    }

    fn fail_translate(&self, message: String) {
        let flash = {
            let mut s = self.state.write();
            s.set_error(message);
            let flash = !s.attention_sent;
            s.attention_sent = true;
            flash
        };
        if flash {
            flash_control_window_taskbar();
        }
    }

    /// Paint the in-flight page with `pairs` streaming in.
    pub(crate) fn apply_stream_preview(&mut self, pairs: &[(u32, String)]) {
        let Some(job) = self.inflight.as_ref() else {
            return;
        };
        let (preview, w, h) = (job.preview(pairs), job.page.content_width, job.page.content_height);
        self.show_captions(preview, w, h);
    }

    /// Map captions onto the current OCR and show them. Does not finish the job or append
    /// history, so it also paints cache hits and stream previews.
    fn show_captions(&mut self, translated: Vec<TranslatedBlock>, content_width: u32, content_height: u32) {
        // The page may have changed or the window resized while translating.
        let (w, h) = self
            .last_raw_ocr
            .as_ref()
            .map_or((content_width, content_height), |c| (c.width, c.height));
        let resized = (w, h) != (content_width, content_height);
        let remapped = remap_translations_to_ocr(&translated, &self.ocr_blocks, resized);
        self.paint_captions(remapped, w, h);
    }

    /// Show a finished page and mark the translation done.
    fn apply_translated(&mut self, translated: Vec<TranslatedBlock>, content_width: u32, content_height: u32) {
        self.show_captions(translated, content_width, content_height);
        let mut s = self.state.write();
        s.last_error = None;
        // Not OverlayActive when nothing was kept (the page left the screen).
        s.restore_operational_status();
        s.attention_sent = false;
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use parking_lot::RwLock;
    use translator_core::{AppConfig, AppState, ModelProvider, Rect};
    use translator_ocr::{BlockPersistenceFilter, StabilityGate};
    use translator_translate::{Completion, merge_translations};

    use super::*;
    use crate::pipeline::worker::{LastRawOcr, OcrJob};

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

    fn pipeline() -> Pipeline {
        let mut config = AppConfig::default();
        config.api.max_retries = 0;
        // Not the built-in default: a spawned job must never download or run a model.
        config.api.provider = ModelProvider::OpenaiCompatible;
        Pipeline::detached(Arc::new(RwLock::new(AppState::new(config))))
    }

    /// A model reply for the in-flight page, as the job would hand it back.
    fn reply(p: &Pipeline, job: &InflightTranslate, json: &str) -> PageResult {
        PageResult {
            conversation: p.conversation.clone(),
            translated: merge_translations(&job.page.blocks, json).expect("valid reply"),
            error: None,
        }
    }

    fn finish(p: &mut Pipeline, json: &str) {
        let job = p.take_inflight().expect("job");
        let result = reply(p, &job, json);
        p.finish_translate(job, result);
    }

    fn page(blocks: Vec<OcrBlock>) -> PendingPage {
        PendingPage {
            blocks,
            content_width: 100,
            content_height: 100,
        }
    }

    #[tokio::test]
    async fn cancelled_job_does_not_block_the_next_page() {
        let mut p = pipeline();
        p.persist = BlockPersistenceFilter::new(0, 750, 0);
        let x = vec![block(1, "X")];
        p.consume_raw_ocr(x.clone(), 100, 100);
        p.consume_raw_ocr(x, 100, 100);
        assert!(p.inflight.is_some(), "page translates");

        // A config or in-use glossary change cancels the request.
        p.cancel_inflight();
        assert!(!p.state.read().translate_in_flight);
        assert!(!p.state.read().status.is_translating());

        let y = vec![block(1, "Y")];
        p.consume_raw_ocr(y.clone(), 100, 100);
        p.consume_raw_ocr(y, 100, 100);
        assert!(p.inflight.is_some(), "the next page translates");
    }

    #[tokio::test]
    async fn history_matches_the_trimmed_model_context() {
        let mut p = pipeline();
        let tcfg = p.state.read().config.translation.clone();
        let answer = Completion {
            text: r#"{"b":[[1,"T"]]}"#.into(),
            replay_items: Vec::new(),
        };
        for i in 0..tcfg.conversation_max_turns {
            let _ = p.conversation.begin_translate_request(&tcfg, &[block(1, &i.to_string())], &[]);
            p.conversation.commit_completion(&answer);
            p.state.write().push_history(i.to_string(), "T".into(), tcfg.conversation_max_turns);
        }

        p.start_translate(page(vec![block(1, "new")]), false);
        let job = p.take_inflight().expect("job");
        // The request trims the context to history_max_items turns, then adds this reply.
        let mut answered = p.conversation.clone();
        let _ = answered.begin_translate_request(&tcfg, &job.page.blocks, &[]);
        answered.commit_completion(&answer);
        let result = PageResult {
            translated: merge_translations(&job.page.blocks, &answer.text).unwrap(),
            conversation: answered,
            error: None,
        };
        p.finish_translate(job, result);

        let s = p.state.read();
        assert_eq!(s.history.len(), tcfg.history_max_items + 1, "one row per turn left in the context");
        assert_eq!(s.history[0].source_text, "new");
    }

    /// Start an auto OCR job that returns `blocks`.
    fn spawn_ocr(p: &mut Pipeline, blocks: Vec<OcrBlock>) {
        p.ocr_job = Some(OcrJob {
            handle: tokio::spawn(async move { Ok(blocks) }),
            started: std::time::Instant::now(),
            sequence: 1,
            width: 100,
            height: 100,
            scope: Vec::new(),
            manual: false,
            stale: false,
        });
    }

    async fn land_ocr(p: &mut Pipeline) {
        let result = (&mut p.ocr_job.as_mut().expect("job").handle).await;
        p.finish_ocr(result);
    }

    #[tokio::test]
    async fn ocr_result_is_dropped_after_a_reset_and_kept_off_the_page_while_paused() {
        let mut p = pipeline();
        let x = vec![block(1, "X")];
        spawn_ocr(&mut p, x.clone());
        p.reset_ocr_session(false);
        land_ocr(&mut p).await;
        assert!(p.last_raw_ocr.is_none(), "a result from before the reset is dropped");
        assert!(p.ocr_blocks.is_empty());

        {
            let mut s = p.state.write();
            s.auto_running = true;
            s.capture_paused = true;
            s.status = PipelineStatus::Paused;
        }
        spawn_ocr(&mut p, x);
        land_ocr(&mut p).await;
        assert!(p.last_raw_ocr.is_some(), "the frame is still cached");
        assert_eq!(p.state.read().status, PipelineStatus::Paused);
    }

    #[tokio::test]
    async fn queued_manual_capture_is_not_preempted_by_an_auto_translate() {
        let mut p = pipeline();
        p.persist = BlockPersistenceFilter::new(0, 750, 0);
        let x = vec![block(1, "X")];
        p.consume_raw_ocr(x.clone(), 100, 100);
        // The next auto result would make the page stable; Retry was pressed while it ran.
        spawn_ocr(&mut p, x);
        p.manual_queued = true;
        land_ocr(&mut p).await;
        assert!(p.inflight.is_none(), "no auto translate in front of the manual capture");
        assert!(p.last_raw_ocr.is_some(), "the frame is still cached");
        assert!(!p.manual_queued);
    }

    #[tokio::test]
    async fn closed_target_stops_the_capture() {
        let mut p = pipeline();
        // No stream is running: the target window was closed.
        p.state.write().auto_running = true;
        p.tick().await;
        let s = p.state.read();
        assert!(!s.auto_running);
        assert_matches!(s.status, PipelineStatus::Error { .. });
        assert!(p.next_tick.is_none(), "no more ticks");
    }

    #[tokio::test]
    async fn reset_during_translation_keeps_the_new_session_empty() {
        let mut p = pipeline();
        p.start_translate(page(vec![block(1, "A")]), false);
        let job = p.take_inflight().expect("job");
        let mut answered = p.conversation.clone();
        let _ = answered.begin_translate_request(&p.state.read().config.translation, &job.page.blocks, &[]);
        answered.commit_completion(&Completion {
            text: r#"{"b":[[1,"T"]]}"#.into(),
            replay_items: Vec::new(),
        });

        p.reset_conversation();
        let result = PageResult {
            translated: merge_translations(&job.page.blocks, r#"{"b":[[1,"T"]]}"#).unwrap(),
            conversation: answered,
            error: None,
        };
        p.finish_translate(job, result);
        assert_eq!(p.conversation.turn_count(), 0, "the old session's turn is not written back");
        assert!(p.state.read().history.is_empty(), "nor is its history row");
        assert_eq!(p.captions.len(), 1, "the translation still shows");
    }

    #[tokio::test]
    async fn vanished_line_is_captioned_again_on_return() {
        let mut p = pipeline();
        // Settle off: durable OCR is the raw frame, so no sleeps.
        p.persist = BlockPersistenceFilter::new(0, 750, 0);
        let both = vec![block(1, "X"), block(2, "Y")];
        p.consume_raw_ocr(both.clone(), 100, 100);
        p.consume_raw_ocr(both.clone(), 100, 100);
        finish(&mut p, r#"{"b":[[1,"TX"],[2,"TY"]]}"#);
        assert_eq!(p.captions.len(), 2);

        p.consume_raw_ocr(vec![block(1, "X")], 100, 100);
        assert_eq!(p.captions.len(), 1, "vanished line drops its caption");

        p.consume_raw_ocr(both.clone(), 100, 100);
        p.consume_raw_ocr(both, 100, 100);
        assert_eq!(p.captions.len(), 2, "returning line is captioned again");
    }

    #[tokio::test]
    async fn result_only_paints_lines_still_on_screen() {
        let mut p = pipeline();
        p.persist = BlockPersistenceFilter::new(0, 750, 0);
        let page = vec![block(1, "X"), block(2, "Y")];
        p.consume_raw_ocr(page.clone(), 100, 100);
        p.consume_raw_ocr(page, 100, 100);
        assert!(p.inflight.is_some(), "page translates");

        // Y is replaced while waiting: its caption must not land on the new text.
        let changed = vec![block(1, "X"), block(2, "Z")];
        p.consume_raw_ocr(changed.clone(), 100, 100);
        p.consume_raw_ocr(changed.clone(), 100, 100);
        finish(&mut p, r#"{"b":[[1,"TX"],[2,"TY"]]}"#);
        assert_eq!(p.captions.iter().map(|b| b.translation.as_str()).collect::<Vec<_>>(), ["TX"]);
        assert_eq!(p.translation_cache.len(), 2, "the hidden line is still cached");
        assert_eq!(p.state.read().latest_translated_blocks.len(), 1, "the UI sees the same captions");

        // The gate saw nothing while busy, so the changed page still translates.
        p.consume_raw_ocr(changed.clone(), 100, 100);
        p.consume_raw_ocr(changed, 100, 100);
        let job = p.inflight.as_ref().expect("changed page translates");
        assert_eq!(job.cached_hits, [Some("TX".to_string()), None], "only Z goes to the model");
    }

    #[tokio::test]
    async fn page_gone_during_translation_shows_from_cache_on_return() {
        let mut p = pipeline();
        p.persist = BlockPersistenceFilter::new(0, 750, 0);
        let x = vec![block(1, "X")];
        p.consume_raw_ocr(x.clone(), 100, 100);
        p.consume_raw_ocr(x.clone(), 100, 100);
        let b = vec![block(1, "B")];
        p.consume_raw_ocr(b.clone(), 100, 100);
        p.consume_raw_ocr(b.clone(), 100, 100);
        finish(&mut p, r#"{"b":[[1,"TX"]]}"#);
        assert!(p.captions.is_empty(), "nothing painted over new text");
        assert_ne!(p.state.read().status, PipelineStatus::OverlayActive, "no captions, no overlay status");

        // The gate saw nothing while busy, so the new page still translates.
        p.consume_raw_ocr(b.clone(), 100, 100);
        p.consume_raw_ocr(b, 100, 100);
        finish(&mut p, r#"{"b":[[1,"TB"]]}"#);

        p.consume_raw_ocr(x.clone(), 100, 100);
        p.consume_raw_ocr(x, 100, 100);
        assert_eq!(p.captions[0].translation, "TX");
    }

    #[tokio::test]
    async fn blank_screen_clears_pending_captions_after_one_hold() {
        let mut p = pipeline();
        p.state.write().config.ocr.hold_ms = 20;
        p.persist = BlockPersistenceFilter::new(20, 20, 0);
        p.gate = StabilityGate::new(0);
        let settle = || std::thread::sleep(std::time::Duration::from_millis(30));
        let x = vec![block(1, "X")];
        p.consume_raw_ocr(x.clone(), 100, 100);
        settle();
        p.consume_raw_ocr(x.clone(), 100, 100);
        p.consume_raw_ocr(x, 100, 100);
        assert!(p.inflight.is_some(), "page translates");
        // The settle filter renumbers block ids from 0.
        p.apply_stream_preview(&[(0, "TX".into())]);
        assert_eq!(p.captions.len(), 1);

        p.consume_raw_ocr(Vec::new(), 100, 100);
        assert_eq!(p.captions.len(), 1, "held for hold_ms");
        settle();
        p.consume_raw_ocr(Vec::new(), 100, 100);
        assert!(p.captions.is_empty(), "cleared once the hold elapses");
        assert_eq!(p.state.read().status, PipelineStatus::Translating, "the blank reset keeps the translating status");
    }

    #[tokio::test]
    async fn replaced_line_caption_expires_after_hold_while_waiting() {
        let mut p = pipeline();
        // Settle longer than hold: the new line is still settling when the hold runs out.
        p.persist = BlockPersistenceFilter::new(60, 30, 0);
        p.gate = StabilityGate::new(0);
        let sleep = |ms| std::thread::sleep(std::time::Duration::from_millis(ms));
        let x = vec![block(1, "アオイ")];
        p.consume_raw_ocr(x.clone(), 100, 100);
        sleep(70);
        p.consume_raw_ocr(x.clone(), 100, 100);
        p.consume_raw_ocr(x, 100, 100);
        // The settle filter renumbers block ids from 0.
        finish(&mut p, r#"{"b":[[0,"Aoi"]]}"#);
        assert_eq!(p.captions.len(), 1);

        let y = vec![block(1, "ハル")];
        p.consume_raw_ocr(y.clone(), 100, 100);
        assert_eq!(p.captions.len(), 1, "held for hold");
        sleep(40);
        p.consume_raw_ocr(y, 100, 100);
        assert!(p.captions.is_empty(), "the replaced line's caption expires after hold");
    }

    #[tokio::test]
    async fn unsettled_text_still_translates_after_max_wait() {
        use std::time::{Duration, Instant};
        let mut p = pipeline();
        {
            // Persist shares max_wait, so it force-adopts the counter's readings too.
            let mut s = p.state.write();
            let ocr = &mut s.config.ocr;
            (ocr.settle_ms, ocr.hold_ms, ocr.max_wait_ms) = (20, 30, 50);
            p.persist = BlockPersistenceFilter::from_config(ocr);
        }
        p.gate = StabilityGate::new(0);
        // X settles; a counter beside it changes every frame.
        let frame = |n: usize| {
            vec![block(1, "X"), OcrBlock {
                bbox: Rect::new(0.0, 60.0, 10.0, 10.0),
                ..block(2, &format!("n{n}"))
            }]
        };
        let start = Instant::now();
        let mut n = 0;
        while start.elapsed() < Duration::from_millis(30) {
            p.consume_raw_ocr(frame(n), 100, 100);
            n += 1;
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(p.inflight.is_none(), "waits while the counter flickers");

        while p.inflight.is_none() && start.elapsed() < Duration::from_millis(400) {
            p.consume_raw_ocr(frame(n), 100, 100);
            n += 1;
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(p.inflight.is_some(), "max_wait sends the settled text anyway");
    }

    #[tokio::test]
    async fn wait_all_settled_off_translates_settled_text_without_max_wait() {
        use std::time::{Duration, Instant};
        for wait_all in [true, false] {
            let mut p = pipeline();
            {
                let mut s = p.state.write();
                let ocr = &mut s.config.ocr;
                (ocr.settle_ms, ocr.hold_ms, ocr.max_wait_ms, ocr.wait_all_settled) = (20, 30, 0, wait_all);
                p.persist = BlockPersistenceFilter::from_config(ocr);
            }
            p.gate = StabilityGate::new(0);
            // X settles; with max_wait 0 the counter beside it never does.
            let frame = |n: usize| {
                vec![block(1, "X"), OcrBlock {
                    bbox: Rect::new(0.0, 60.0, 10.0, 10.0),
                    ..block(2, &format!("n{n}"))
                }]
            };
            let start = Instant::now();
            let mut n = 0;
            while p.inflight.is_none() && start.elapsed() < Duration::from_millis(150) {
                p.consume_raw_ocr(frame(n), 100, 100);
                n += 1;
                std::thread::sleep(Duration::from_millis(5));
            }
            assert_eq!(p.inflight.is_some(), !wait_all, "wait_all_settled = {wait_all}");
            if !wait_all {
                let texts: Vec<_> = p.last_page.as_ref().expect("page").blocks.iter().map(|b| b.text.as_str()).collect();
                assert_eq!(texts, ["X"]);
            }
        }
    }

    #[tokio::test]
    async fn cached_settled_line_shows_while_the_page_waits() {
        use std::time::Duration;
        let mut p = pipeline();
        {
            let mut s = p.state.write();
            let ocr = &mut s.config.ocr;
            (ocr.settle_ms, ocr.hold_ms, ocr.max_wait_ms) = (20, 30, 0);
            p.persist = BlockPersistenceFilter::from_config(ocr);
            let tr = &s.config.translation;
            p.translation_cache.insert("X", &tr.source_lang, &tr.target_lang, "TX");
        }
        p.gate = StabilityGate::new(0);
        // X settles; the counter beside it never does, so the page waits for good.
        let frame = |n: usize| {
            vec![block(1, "X"), OcrBlock {
                bbox: Rect::new(0.0, 60.0, 10.0, 10.0),
                ..block(2, &format!("n{n}"))
            }]
        };
        for n in 0..12 {
            p.consume_raw_ocr(frame(n), 100, 100);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(p.inflight.is_none(), "the page still waits");
        let shown: Vec<_> = p.captions.iter().map(|c| c.translation.as_str()).collect();
        assert_eq!(shown, ["TX"], "the cached settled line shows anyway");
    }

    #[tokio::test]
    async fn held_line_is_not_mixed_into_the_next_page() {
        let mut p = pipeline();
        p.state.write().config.ocr.max_wait_ms = 50;
        p.persist = BlockPersistenceFilter::new(20, 60, 0);
        p.gate = StabilityGate::new(0);
        let sleep = |ms| std::thread::sleep(std::time::Duration::from_millis(ms));
        let at = |text: &str, y: f32| {
            vec![OcrBlock {
                bbox: Rect::new(0.0, y, 10.0, 10.0),
                ..block(1, text)
            }]
        };
        let a = at("A", 0.0);
        p.consume_raw_ocr(a.clone(), 100, 100);
        sleep(30);
        p.consume_raw_ocr(a.clone(), 100, 100);
        p.consume_raw_ocr(a.clone(), 100, 100);
        finish(&mut p, r#"{"b":[[0,"TA"]]}"#);
        // A stays up past max_wait: max_wait counts from the change, not from the last page.
        sleep(55);
        p.consume_raw_ocr(a, 100, 100);

        // A leaves and B shows up elsewhere: B settles while A is still held by hysteresis.
        let b = at("B", 80.0);
        p.consume_raw_ocr(b.clone(), 100, 100);
        sleep(30);
        p.consume_raw_ocr(b.clone(), 100, 100);
        p.consume_raw_ocr(b.clone(), 100, 100);
        assert!(p.inflight.is_none(), "no page while A is still held");

        sleep(40);
        p.consume_raw_ocr(b.clone(), 100, 100);
        p.consume_raw_ocr(b, 100, 100);
        assert!(p.inflight.is_some(), "B translates once A expired");
        let texts: Vec<_> = p.last_page.as_ref().expect("page").blocks.iter().map(|b| b.text.as_str()).collect();
        assert_eq!(texts, ["B"]);
    }

    #[tokio::test]
    async fn result_lands_in_the_resized_pixel_space() {
        let mut p = pipeline();
        p.persist = BlockPersistenceFilter::new(0, 750, 0);
        let x = vec![block(1, "X")];
        p.consume_raw_ocr(x.clone(), 100, 100);
        p.consume_raw_ocr(x, 100, 100);
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
        p.consume_raw_ocr(vec![big.clone()], 200, 200);
        finish(&mut p, r#"{"b":[[1,"TX"]]}"#);
        assert_eq!(p.captions[0].bbox, big.bbox);
    }

    #[tokio::test]
    async fn unsettled_ocr_during_translation_keeps_page_paintable() {
        let mut p = pipeline();
        p.persist = BlockPersistenceFilter::new(20, 20, 0);
        p.gate = StabilityGate::new(0);
        let x = vec![block(1, "X")];
        p.consume_raw_ocr(x.clone(), 100, 100);
        std::thread::sleep(std::time::Duration::from_millis(30));
        p.consume_raw_ocr(x.clone(), 100, 100);
        p.consume_raw_ocr(x.clone(), 100, 100);
        assert!(p.inflight.is_some(), "page translates");

        // A manual capture resets persist: nothing is settled while the request runs.
        p.persist.reset();
        p.consume_raw_ocr(x.clone(), 100, 100);
        // The settle filter renumbers block ids from 0.
        p.apply_stream_preview(&[(0, "TX".into())]);
        assert_eq!(p.captions.len(), 1, "stream preview still paints");
        p.consume_raw_ocr(x.clone(), 100, 100);
        assert_eq!(p.captions.len(), 1, "a reset persist does not drop the preview");

        finish(&mut p, r#"{"b":[[0,"TX"]]}"#);
        p.consume_raw_ocr(x.clone(), 100, 100);
        assert_eq!(p.captions.len(), 1, "nor the result");
        std::thread::sleep(std::time::Duration::from_millis(30));
        p.consume_raw_ocr(x.clone(), 100, 100);
        p.consume_raw_ocr(x, 100, 100);
        assert!(p.inflight.is_none(), "the page is not translated again once it settles");
    }

    #[tokio::test]
    async fn page_returning_after_hold_expiry_is_translated_again() {
        let mut p = pipeline();
        p.state.write().config.ocr.hold_ms = 20;
        p.persist = BlockPersistenceFilter::new(20, 20, 0);
        p.gate = StabilityGate::new(0);
        let settle = || std::thread::sleep(std::time::Duration::from_millis(30));
        let x = vec![block(1, "X")];
        p.consume_raw_ocr(x.clone(), 100, 100);
        settle();
        p.consume_raw_ocr(x.clone(), 100, 100);
        p.consume_raw_ocr(x.clone(), 100, 100);
        // The settle filter renumbers block ids from 0.
        finish(&mut p, r#"{"b":[[0,"TX"]]}"#);
        assert_eq!(p.captions.len(), 1);

        // Dialog closes over flickering text that never settles: captions expire after hold.
        let noise = |text: &str| {
            vec![OcrBlock {
                bbox: Rect::new(60.0, 60.0, 10.0, 10.0),
                ..block(1, text)
            }]
        };
        settle();
        p.consume_raw_ocr(noise("n1"), 100, 100);
        settle();
        p.consume_raw_ocr(noise("n2"), 100, 100);
        assert!(p.captions.is_empty());

        p.consume_raw_ocr(x.clone(), 100, 100);
        settle();
        p.consume_raw_ocr(x.clone(), 100, 100);
        p.consume_raw_ocr(x, 100, 100);
        assert_eq!(p.captions.len(), 1, "same page is translated again");
    }
}

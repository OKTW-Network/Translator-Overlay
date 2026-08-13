//! Translate job lifecycle (start, finish, apply to overlay / state).

use std::sync::Arc;

use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use translator_core::{PipelineStatus, TranslatedBlock};
use translator_overlay::OverlayController;
use translator_translate::{TranslateError, blocks_to_translated_text, merge_translations};

use crate::pipeline::{
    wake::NotifyOnDrop,
    worker::{InflightTranslate, PendingPage, Pipeline, SharedState, TranslateJobResult},
};

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

        // Canonical conversation prepare (shared with translate_blocks_*).
        let (messages, messages_len_after_user) = self.conversation.begin_translate_request(&tcfg, &page.blocks);

        let cancel = CancellationToken::new();
        let cancel_job = cancel.clone();
        let client_clone = self.client.clone();
        let wake = Arc::clone(&self.wake);
        let (tx, rx) = oneshot::channel();

        {
            let mut s = self.state.write();
            s.status = PipelineStatus::Translating;
            s.translate_in_flight = true;
            s.last_error = None;
            s.can_retry_translate = true;
        }

        self.rt.spawn(async move {
            let _notify = NotifyOnDrop(wake);
            let result = client_clone.chat_completions_with_retry(&messages, &cancel_job).await;
            let _ = tx.send(TranslateJobResult {
                result,
                messages_len_after_user,
            });
        });

        self.inflight = Some(InflightTranslate {
            cancel,
            rx,
            fingerprint: page.fingerprint,
            blocks: page.blocks,
            source_text: page.source_text,
            content_width: page.content_width,
            content_height: page.content_height,
            messages_len_after_user,
        });
    }

    pub(crate) fn finish_translate(&mut self, job: InflightTranslate, job_result: TranslateJobResult) {
        match job_result.result {
            Ok(content) => match merge_translations(&job.blocks, &content) {
                Ok(translated) => {
                    self.conversation.push_assistant(&content);
                    let translated_text = blocks_to_translated_text(&translated);
                    info!(blocks = translated.len(), turns = self.conversation.turn_count, "translation complete");
                    self.last_translated_fp = Some(job.fingerprint);
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
                    self.conversation.rollback_user_turn(job_result.messages_len_after_user);
                    let mut s = self.state.write();
                    s.translate_in_flight = false;
                    s.can_retry_translate = true;
                    s.set_error(format!("translate parse: {e}"));
                }
            },
            Err(e) if e.is_cancelled() => {
                info!("translation cancelled");
                self.conversation.rollback_user_turn(job_result.messages_len_after_user);
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
                self.conversation.rollback_user_turn(job_result.messages_len_after_user);
                let mut s = self.state.write();
                s.translate_in_flight = false;
                s.can_retry_translate = true;
                s.set_error(format!("translate: {e}"));
            }
        }
    }

    pub(crate) fn poll_translate(&mut self) {
        let Some(job) = self.inflight.as_mut() else {
            return;
        };
        match job.rx.try_recv() {
            Ok(job_result) => {
                let finished = self.inflight.take().expect("inflight present");
                self.finish_translate(finished, job_result);
            }
            Err(oneshot::error::TryRecvError::Empty) => {}
            Err(oneshot::error::TryRecvError::Closed) => {
                let finished = self.inflight.take().expect("inflight present");
                let messages_len_after_user = finished.messages_len_after_user;
                self.finish_translate(finished, TranslateJobResult {
                    result: Err(TranslateError::Other("translate task dropped".into())),
                    messages_len_after_user,
                });
            }
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

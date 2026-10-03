//! Runtime application / pipeline state.

use std::collections::VecDeque;

use bytes::Bytes;

use crate::{
    config::AppConfig,
    types::{NormRect, OcrBlock, TranslatedBlock},
};

/// High-level pipeline status shown in the control UI.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum PipelineStatus {
    #[default]
    Idle,
    Capturing,
    RunningOcr,
    WaitingForStable {
        elapsed_ms: u64,
    },
    /// App-owned model download (GitHub → `models_dir`); UI stays operable.
    DownloadingModels {
        file: String,
        /// 1-based index among files being fetched this run.
        file_index: u32,
        file_count: u32,
        percent: u8,
    },
    /// Building the ONNX Runtime session after files are on disk.
    LoadingModels,
    Translating,
    /// Auto-retry after a transient translate API / network / CLI error.
    RetryingTranslate {
        /// 1-based retry about to run.
        attempt: u32,
        max_retries: u32,
        /// Display of the error that triggered this retry.
        message: String,
    },
    /// In-flight translation was aborted by the user.
    Cancelled,
    OverlayActive,
    Error {
        message: String,
    },
}

/// Token stored in [`AppState::settings_message`] after a successful save.
/// The control window translates it; do not localize this value.
pub const SETTINGS_SAVED: &str = "Saved";

impl PipelineStatus {
    pub fn is_translating(&self) -> bool {
        matches!(self, Self::Translating | Self::RetryingTranslate { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_history_caps_to_max_and_assigns_ids() {
        let mut state = AppState::new(AppConfig::default());
        for i in 0..4 {
            state.push_history(format!("s{i}"), format!("t{i}"), 3);
        }
        let ids: Vec<u64> = state.history.iter().map(|h| h.id).collect();
        assert_eq!(ids, vec![3, 2, 1]);
        assert_eq!(state.history.front().unwrap().source_text, "s3");
        assert_eq!(state.history.back().unwrap().source_text, "s1");
    }

    #[test]
    fn history_entry_covered_by_live_page() {
        let entry = HistoryEntry {
            id: 1,
            source_text: "new line".into(),
            translated_text: "新行".into(),
        };
        assert!(entry.covered_by("cached\nnew line", "快取\n新行"));
        assert!(entry.covered_by("new line", "新行"));
        assert!(!entry.covered_by("cached", "快取"));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HistoryEntry {
    /// Stable UI key; assigned by [`AppState::push_history`].
    pub id: u64,
    pub source_text: String,
    pub translated_text: String,
}

impl HistoryEntry {
    /// True when every line of this API turn already appears on the live page.
    pub fn covered_by(&self, live_source: &str, live_translation: &str) -> bool {
        lines_subset(&self.source_text, live_source) && lines_subset(&self.translated_text, live_translation)
    }
}

fn lines_subset(part: &str, whole: &str) -> bool {
    part.lines()
        .filter(|line| !line.is_empty())
        .all(|line| whole.lines().any(|w| w == line))
}

/// Capture thumbnail shared with the control UI.
#[derive(Debug, Clone, Default)]
pub struct PreviewInfo {
    pub width: u32,
    pub height: u32,
    pub sequence: u64,
    /// Tightly packed RGBA8 (`width * height * 4`).
    pub rgba: Option<Bytes>,
}

/// Mutable runtime state shared between UI and workers.
#[derive(Debug)]
pub struct AppState {
    pub status: PipelineStatus,
    pub config: AppConfig,
    pub latest_ocr_blocks: Vec<OcrBlock>,
    pub latest_translated_blocks: Vec<TranslatedBlock>,
    pub history: VecDeque<HistoryEntry>,
    pub target_window_title: Option<String>,
    pub target_hwnd: Option<isize>,
    pub preview: PreviewInfo,
    pub auto_running: bool,
    /// Start/Stop locked while Stop waits for CLI session close.
    pub capture_busy: bool,
    /// True while an LLM request is in flight (cancellable).
    pub translate_in_flight: bool,
    /// Last error message (kept after status changes so the UI can show it).
    pub last_error: Option<String>,
    /// Settings last saved successfully (shown in UI).
    pub settings_message: Option<String>,
    /// Wall-clock duration of the last OCR inference (ms), if any.
    pub last_ocr_ms: Option<u64>,
    /// Number of text blocks from the last OCR pass (pre-merge raw or durable).
    pub last_ocr_block_count: u32,
    /// Session OCR crops (normalized client rects). Empty = whole window.
    pub ocr_regions: Vec<NormRect>,
    /// True while the on-target region picker is open.
    pub region_select_active: bool,
    /// Working copy while picking (for Dashboard count / preview outlines).
    pub region_select_draft: Vec<NormRect>,
    /// Unique source strings currently in the session translation cache.
    pub translation_cache_len: usize,
    /// True after a translate failure until a translate succeeds (one flash per streak).
    pub attention_sent: bool,
    next_history_id: u64,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            status: PipelineStatus::Idle,
            config,
            latest_ocr_blocks: Vec::new(),
            latest_translated_blocks: Vec::new(),
            history: VecDeque::new(),
            target_window_title: None,
            target_hwnd: None,
            preview: PreviewInfo::default(),
            auto_running: false,
            capture_busy: false,
            translate_in_flight: false,
            last_error: None,
            settings_message: None,
            last_ocr_ms: None,
            last_ocr_block_count: 0,
            ocr_regions: Vec::new(),
            region_select_active: false,
            region_select_draft: Vec::new(),
            translation_cache_len: 0,
            attention_sent: false,
            next_history_id: 0,
        }
    }

    pub fn set_error(&mut self, message: impl Into<String>) {
        let message = message.into();
        self.last_error = Some(message.clone());
        self.status = PipelineStatus::Error { message };
    }

    /// Restore a non-error operational status from current flags / overlay content.
    ///
    /// Shared by config save, conversation reset, and similar UI-side recoveries
    /// so status rules stay in one place.
    pub fn restore_operational_status(&mut self) {
        if self.translate_in_flight {
            self.status = PipelineStatus::Translating;
        } else if self.auto_running {
            self.status = PipelineStatus::Capturing;
        } else if !self.latest_translated_blocks.is_empty() {
            self.status = PipelineStatus::OverlayActive;
        } else {
            self.status = PipelineStatus::Idle;
        }
    }

    pub fn push_history(&mut self, source_text: String, translated_text: String, max_items: usize) {
        let id = self.next_history_id;
        self.next_history_id += 1;
        self.history.push_front(HistoryEntry {
            id,
            source_text,
            translated_text,
        });
        self.history.truncate(max_items.max(1));
    }
}

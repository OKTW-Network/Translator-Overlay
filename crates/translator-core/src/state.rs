//! Runtime application / pipeline state.

use std::collections::VecDeque;

use bytes::Bytes;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    config::AppConfig,
    types::{NormRect, OcrBlock, TranslatedBlock},
};

/// High-level pipeline status shown in the control UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PipelineStatus {
    #[default]
    Idle,
    SelectingWindow,
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

impl PipelineStatus {
    pub fn label(&self) -> String {
        match self {
            Self::Idle => "Idle".to_string(),
            Self::SelectingWindow => "Selecting window".to_string(),
            Self::Capturing => "Capturing".to_string(),
            Self::RunningOcr => "Running OCR".to_string(),
            Self::WaitingForStable { elapsed_ms } => {
                format!("Waiting for stable text ({elapsed_ms} ms)")
            }
            Self::DownloadingModels {
                file,
                file_index,
                file_count,
                percent,
            } => format!("Downloading OCR models ({file_index}/{file_count} · {file} · {percent}%)"),
            Self::LoadingModels => "Loading OCR models".to_string(),
            Self::Translating => "Translating".to_string(),
            Self::RetryingTranslate { attempt, max_retries, .. } => {
                format!("Retrying translation ({attempt}/{max_retries})")
            }
            Self::Cancelled => "Cancelled".to_string(),
            Self::OverlayActive => "Overlay active".to_string(),
            Self::Error { message } => format!("Error: {message}"),
        }
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. })
    }

    pub fn is_translating(&self) -> bool {
        matches!(self, Self::Translating | Self::RetryingTranslate { .. })
    }

    pub fn is_busy(&self) -> bool {
        // Download / ORT load are informational — do not lock the UI.
        matches!(self, Self::RunningOcr | Self::Translating | Self::RetryingTranslate { .. } | Self::WaitingForStable { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retrying_status_label_is_compact() {
        let status = PipelineStatus::RetryingTranslate {
            attempt: 1,
            max_retries: 2,
            message: "API returned status 429: {\"error\":\"rate\"}".into(),
        };
        assert_eq!(status.label(), "Retrying translation (1/2)");
        assert!(status.is_translating());
        assert!(status.is_busy());
        assert!(!status.is_error());
    }

    #[test]
    fn translating_is_busy_and_translating() {
        assert!(PipelineStatus::Translating.is_translating());
        assert!(PipelineStatus::Translating.is_busy());
        assert!(!PipelineStatus::Capturing.is_translating());
    }

    #[test]
    fn download_and_load_are_not_busy() {
        let downloading = PipelineStatus::DownloadingModels {
            file: "pp-ocrv6_small_det.onnx".into(),
            file_index: 1,
            file_count: 3,
            percent: 42,
        };
        assert_eq!(downloading.label(), "Downloading OCR models (1/3 · pp-ocrv6_small_det.onnx · 42%)");
        assert!(!downloading.is_busy());
        assert!(!PipelineStatus::LoadingModels.is_busy());
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: u64,
    pub timestamp: DateTime<Utc>,
    pub source_text: String,
    pub translated_text: String,
    pub blocks: Vec<TranslatedBlock>,
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
#[derive(Debug, Clone)]
pub struct AppState {
    pub status: PipelineStatus,
    pub config: AppConfig,
    pub latest_ocr_text: String,
    pub latest_ocr_blocks: Vec<OcrBlock>,
    pub latest_translated_text: String,
    pub latest_translated_blocks: Vec<TranslatedBlock>,
    pub history: VecDeque<HistoryEntry>,
    pub target_window_title: Option<String>,
    pub target_hwnd: Option<isize>,
    pub preview: PreviewInfo,
    pub frame_count: u64,
    pub auto_running: bool,
    pub next_history_id: u64,
    /// True while an LLM request is in flight (cancellable).
    pub translate_in_flight: bool,
    /// Last error message (kept after status changes so the UI can show it).
    pub last_error: Option<String>,
    /// True when the latest OCR page can be re-sent to the translator.
    pub can_retry_translate: bool,
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
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            status: PipelineStatus::Idle,
            config,
            latest_ocr_text: String::new(),
            latest_ocr_blocks: Vec::new(),
            latest_translated_text: String::new(),
            latest_translated_blocks: Vec::new(),
            history: VecDeque::new(),
            target_window_title: None,
            target_hwnd: None,
            preview: PreviewInfo::default(),
            frame_count: 0,
            auto_running: false,
            next_history_id: 1,
            translate_in_flight: false,
            last_error: None,
            can_retry_translate: false,
            settings_message: None,
            last_ocr_ms: None,
            last_ocr_block_count: 0,
            ocr_regions: Vec::new(),
            region_select_active: false,
            region_select_draft: Vec::new(),
            translation_cache_len: 0,
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

    pub fn push_history(&mut self, source_text: String, translated_text: String, blocks: Vec<TranslatedBlock>) {
        let id = self.next_history_id;
        self.next_history_id += 1;
        self.history.push_front(HistoryEntry {
            id,
            timestamp: Utc::now(),
            source_text,
            translated_text,
            blocks,
        });
        // Keep a generous local UI history independent of API context limits.
        const UI_HISTORY_CAP: usize = 200;
        while self.history.len() > UI_HISTORY_CAP {
            self.history.pop_back();
        }
    }
}

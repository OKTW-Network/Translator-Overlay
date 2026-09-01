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

impl PipelineStatus {
    pub fn label(&self) -> String {
        match self {
            Self::Idle => "Idle".to_string(),
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

    pub fn is_translating(&self) -> bool {
        matches!(self, Self::Translating | Self::RetryingTranslate { .. })
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
    }

    #[test]
    fn translating_status() {
        assert!(PipelineStatus::Translating.is_translating());
        assert!(!PipelineStatus::Capturing.is_translating());
    }

    #[test]
    fn download_label() {
        let downloading = PipelineStatus::DownloadingModels {
            file: "pp-ocrv6_small_det.onnx".into(),
            file_index: 1,
            file_count: 3,
            percent: 42,
        };
        assert_eq!(downloading.label(), "Downloading OCR models (1/3 · pp-ocrv6_small_det.onnx · 42%)");
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HistoryEntry {
    pub source_text: String,
    pub translated_text: String,
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
            translate_in_flight: false,
            last_error: None,
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

    pub fn push_history(&mut self, source_text: String, translated_text: String) {
        self.history.push_front(HistoryEntry {
            source_text,
            translated_text,
        });
        // Dashboard Recent pane shows 5 rows.
        while self.history.len() > 5 {
            self.history.pop_back();
        }
    }
}

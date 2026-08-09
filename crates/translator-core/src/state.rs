//! Runtime application / pipeline state.

use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::types::{OcrBlock, TranslatedBlock};

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
    /// OCR engine load (may block while oar-ocr fetches models).
    LoadingModels,
    Translating,
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
            Self::LoadingModels => "Loading OCR models".to_string(),
            Self::Translating => "Translating".to_string(),
            Self::Cancelled => "Cancelled".to_string(),
            Self::OverlayActive => "Overlay active".to_string(),
            Self::Error { message } => format!("Error: {message}"),
        }
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. })
    }

    pub fn is_busy(&self) -> bool {
        matches!(
            self,
            Self::RunningOcr
                | Self::Translating
                | Self::LoadingModels
                | Self::WaitingForStable { .. }
        )
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

/// Lightweight capture preview metadata for the control UI.
#[derive(Debug, Clone, Default)]
pub struct PreviewInfo {
    pub width: u32,
    pub height: u32,
    pub sequence: u64,
    pub path: Option<String>,
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
        }
    }

    pub fn set_error(&mut self, message: impl Into<String>) {
        let message = message.into();
        self.last_error = Some(message.clone());
        self.status = PipelineStatus::Error { message };
    }

    pub fn push_history(
        &mut self,
        source_text: String,
        translated_text: String,
        blocks: Vec<TranslatedBlock>,
    ) {
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

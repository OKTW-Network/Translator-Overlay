//! Background pipeline: capture → OCR → stability gate → LLM translate → overlay.

mod config_apply;
mod ocr;
mod translate;
mod worker;

use std::sync::OnceLock;

use translator_core::{AppConfig, NormRect};

pub use crate::pipeline::worker::{CmdTx, SharedState, spawn_pipeline};

static UI_PING: OnceLock<std::sync::mpsc::Sender<()>> = OnceLock::new();

/// Capture a Send ping so the pipeline can wake the UI component.
/// First call wins; later calls are ignored.
pub fn install_ui_ping(tx: std::sync::mpsc::Sender<()>) {
    let _ = UI_PING.set(tx);
}

/// Request a control-window rerender. No-op until [`install_ui_ping`].
pub fn ping_ui() {
    if let Some(tx) = UI_PING.get() {
        let _ = tx.send(());
    }
}

/// Commands the UI sends to the pipeline worker.
#[derive(Debug)]
pub enum PipelineCommand {
    StartCapture {
        hwnd: isize,
        title: String,
    },
    StopCapture,
    /// Grab one frame and OCR + translate immediately (bypass stability wait).
    ManualCapture,
    /// Drop LLM conversation history (keeps OCR models and translation cache).
    ResetConversation,
    /// Drop the session translation cache (keeps conversation history).
    ClearTranslationCache,
    /// Apply a full config snapshot (saved from the settings UI).
    ApplyConfig(Box<AppConfig>),
    /// Persist and apply overlay / reader visibility without saving the rest of the draft.
    SetOverlayDisplay {
        enabled: bool,
        reader_enabled: bool,
    },
    /// Cancel the in-flight translation request (if any).
    CancelTranslate,
    /// Open the on-target region picker (`hwnd` is the capture / selected window).
    BeginRegionSelect {
        hwnd: isize,
    },
    ConfirmRegionSelect,
    ClearRegionSelect,
    /// Replace session OCR crops (empty = whole window). Not persisted.
    SetCaptureRegions {
        regions: Vec<NormRect>,
    },
    Shutdown,
}

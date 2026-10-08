//! Background pipeline. Frames go from capture to OCR, through the stability gate,
//! to LLM translation, and then to the overlay.

mod config_apply;
mod ocr;
mod translate;
mod worker;

use std::sync::OnceLock;

use translator_core::{AppConfig, NormRect, UiLanguage};

pub use crate::pipeline::worker::{CmdTx, SharedState, spawn_pipeline};

static UI_PING: OnceLock<std::sync::mpsc::Sender<()>> = OnceLock::new();

/// Store a `Send` ping so the pipeline can wake the UI component.
/// The first call wins, and later calls are ignored.
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
    PauseCapture,
    ResumeCapture,
    StopCapture,
    /// The Dashboard window combo changed. Wakes the worker so the HUD Start button can turn on.
    SetSelectedWindow,
    /// Grab one frame, then OCR and translate it right away, skipping the stability wait.
    ManualCapture,
    /// Drop the LLM conversation history. The OCR models and the translation cache stay.
    ResetConversation,
    /// Drop the session translation cache. The conversation history stays.
    ClearTranslationCache,
    /// Apply a full config snapshot saved from the settings UI.
    ApplyConfig(Box<AppConfig>),
    /// Save and apply the overlay and reader visibility without saving the rest of the draft.
    SetOverlayDisplay {
        enabled: bool,
        reader_enabled: bool,
        hud_enabled: bool,
    },
    /// Save the control-window language without saving the rest of the draft.
    SetUiLanguage {
        language: UiLanguage,
    },
    /// Cancel the in-flight translation request, if there is one.
    CancelTranslate,
    /// Open the region picker over the target. `hwnd` is the captured or selected window.
    BeginRegionSelect {
        hwnd: isize,
    },
    ConfirmRegionSelect,
    ClearRegionSelect,
    /// Replace the session's OCR crops. Empty means the whole window. Not saved to disk.
    SetCaptureRegions {
        regions: Vec<NormRect>,
    },
    Shutdown,
}

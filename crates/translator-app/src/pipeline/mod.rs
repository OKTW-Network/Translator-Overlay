//! Background pipeline: capture → OCR → stability gate → LLM translate → overlay.

mod config_apply;
mod ocr;
mod translate;
mod worker;

use std::sync::OnceLock;

use translator_core::{AppConfig, NormRect};
use windows_reactor::{HostId, UiMarshaller, request_ui_rerender_on_ui_thread};

pub use crate::pipeline::worker::{CmdTx, SharedState, spawn_pipeline};

static UI_PING: OnceLock<(UiMarshaller, HostId)> = OnceLock::new();

/// Capture the WinUI marshaller so the pipeline can request a root rerender.
/// First call wins; later calls are ignored.
pub fn install_ui_ping(marshaller: UiMarshaller, host_id: HostId) {
    let _ = UI_PING.set((marshaller, host_id));
}

/// Request a control-window rerender. No-op until [`install_ui_ping`].
pub fn ping_ui() {
    let Some((marshaller, host_id)) = UI_PING.get() else {
        return;
    };
    let host_id = *host_id;
    let _ = marshaller.dispatch(move || {
        request_ui_rerender_on_ui_thread(host_id);
    });
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

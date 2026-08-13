//! Background pipeline: capture → OCR → stability gate → LLM translate → overlay.

mod config_apply;
mod ocr;
mod remap;
mod translate;
mod wake;
mod worker;

use translator_core::AppConfig;

pub use crate::pipeline::worker::{CmdTx, SharedState, spawn_pipeline};

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
    /// Drop LLM conversation history (keeps OCR models).
    ResetConversation,
    /// Apply a full config snapshot (saved from the settings UI).
    ApplyConfig(Box<AppConfig>),
    /// Cancel the in-flight translation request (if any).
    CancelTranslate,
    /// Re-run translation on the latest OCR blocks.
    RetryTranslate,
    Shutdown,
}

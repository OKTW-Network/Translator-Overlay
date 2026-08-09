//! PP-OCRv6 inference via `oar-ocr` (ONNX Runtime) and text stability gate.

mod engine;
mod filter;
mod merge;
mod models;
mod stability;

pub use engine::*;
pub use filter::*;
pub use merge::*;
pub use models::*;
pub use stability::*;

use thiserror::Error;
use translator_core::OcrConfig;

#[derive(Debug, Error)]
pub enum OcrError {
    #[error("OCR engine not loaded")]
    NotLoaded,
    #[error("OCR engine error: {0}")]
    Engine(String),
    #[error("image error: {0}")]
    Image(String),
    #[error(transparent)]
    Path(#[from] translator_core::PathError),
    #[error("{0}")]
    Other(String),
}

/// Load the OCR engine (oar-ocr may fetch missing models into `models_dir`).
pub fn prepare_engine(config: &OcrConfig) -> Result<OcrEngine, OcrError> {
    OcrEngine::load(config)
}

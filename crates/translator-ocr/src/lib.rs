//! PP-OCRv6 inference via `oar-ocr` (ONNX Runtime) and text stability gate.

mod crop;
mod engine;
mod filter;
mod merge;
mod models;
mod stability;

use thiserror::Error;

pub use crate::{crop::*, engine::*, filter::*, merge::*, models::*, stability::*};

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

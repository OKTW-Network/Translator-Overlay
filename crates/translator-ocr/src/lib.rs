//! PP-OCR inference (PaddleOCR's pipeline on ONNX Runtime) and text stability gate.

mod crop;
mod det;
mod download;
mod engine;
mod filter;
mod layout;
mod merge;
mod models;
mod raster;
mod rec;
mod stability;

use thiserror::Error;
use translator_core::{DownloadError, OcrBlock, PathError};

pub use crate::{crop::*, download::*, engine::*, filter::*, merge::*, models::*, stability::*};

pub(crate) fn reindex(mut blocks: Vec<OcrBlock>) -> Vec<OcrBlock> {
    for (i, b) in blocks.iter_mut().enumerate() {
        b.id = i as u32;
    }
    blocks
}

#[derive(Debug, Error)]
pub enum OcrError {
    #[error("OCR engine error: {0}")]
    Engine(String),
    #[error("image error: {0}")]
    Image(String),
    #[error(transparent)]
    Download(#[from] DownloadError),
    #[error(transparent)]
    Path(#[from] PathError),
    #[error("{0}")]
    Other(String),
}

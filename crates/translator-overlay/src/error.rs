//! Overlay crate errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum OverlayError {
    #[error("overlay thread is not running")]
    NotRunning,
    #[error("failed to start overlay thread: {0}")]
    Spawn(String),
    #[error("{0}")]
    Other(String),
}

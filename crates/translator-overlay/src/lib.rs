//! Transparent click-through overlay and an independent translation window.
//!
//! Creates a layered Win32 popup (`WS_EX_LAYERED | WS_EX_TRANSPARENT | …`) that
//! tracks a target window and draws semi-transparent boxes + translations at
//! OCR bounding boxes (mapped from capture-image coordinates). A second,
//! clickable always-on-top reader window shows the same text independently.

mod draw;
mod host;
mod layout;
mod reader;
mod text;

use std::{
    sync::mpsc::{self, Sender},
    thread::JoinHandle,
};

use thiserror::Error;
use tracing::{error, info};
use translator_core::{OverlayConfig, TranslatedBlock};

use crate::host::{OverlayCommand, OverlayHost};

#[derive(Debug, Error)]
pub enum OverlayError {
    #[error("overlay thread is not running")]
    NotRunning,
    #[error("failed to start overlay thread: {0}")]
    Spawn(String),
    #[error("{0}")]
    Other(String),
}

/// Handle to a background overlay window thread.
pub struct OverlayController {
    tx: Sender<OverlayCommand>,
    join: Option<JoinHandle<()>>,
}

impl OverlayController {
    /// Spawn the overlay host thread and create the layered window.
    pub fn spawn(config: OverlayConfig) -> Result<Self, OverlayError> {
        let (tx, rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();

        let join = std::thread::Builder::new()
            .name("overlay".into())
            .spawn(move || match OverlayHost::create(config) {
                Ok(mut host) => {
                    let _ = ready_tx.send(Ok(()));
                    host.run(rx);
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                }
            })
            .map_err(|e| OverlayError::Spawn(e.to_string()))?;

        match ready_rx.recv() {
            Ok(Ok(())) => {
                info!("overlay host ready");
                Ok(Self { tx, join: Some(join) })
            }
            Ok(Err(e)) => {
                let _ = join.join();
                Err(e)
            }
            Err(_) => {
                let _ = join.join();
                Err(OverlayError::Spawn("overlay thread exited before ready".into()))
            }
        }
    }

    /// Follow `target_hwnd` (screen position / size).
    pub fn attach(&self, target_hwnd: isize) -> Result<(), OverlayError> {
        self.send(OverlayCommand::Attach { target_hwnd })
    }

    /// Stop following any target and hide.
    pub fn detach(&self) -> Result<(), OverlayError> {
        self.send(OverlayCommand::Detach)
    }

    /// Replace drawn translation blocks.
    ///
    /// `content_width` / `content_height` are the capture-image dimensions the
    /// block bboxes are expressed in.
    pub fn set_blocks(&self, blocks: Vec<TranslatedBlock>, content_width: u32, content_height: u32) -> Result<(), OverlayError> {
        self.send(OverlayCommand::SetBlocks {
            blocks,
            content_width,
            content_height,
        })
    }

    /// Clear content and hide the overlay.
    pub fn clear(&self) -> Result<(), OverlayError> {
        self.send(OverlayCommand::Clear)
    }

    pub fn update_config(&self, config: OverlayConfig) -> Result<(), OverlayError> {
        self.send(OverlayCommand::UpdateConfig(config))
    }

    /// Request shutdown (also called from `Drop`).
    pub fn shutdown(&mut self) {
        let _ = self.tx.send(OverlayCommand::Shutdown);
        if let Some(join) = self.join.take()
            && let Err(e) = join.join()
        {
            error!(?e, "overlay thread join failed");
        }
    }

    fn send(&self, cmd: OverlayCommand) -> Result<(), OverlayError> {
        self.tx.send(cmd).map_err(|_| OverlayError::NotRunning)
    }
}

impl Drop for OverlayController {
    fn drop(&mut self) {
        self.shutdown();
    }
}

// Re-export pure helpers for tests / callers.
pub use crate::draw::{Rgba, SurfaceRect, SurfaceSize, argb_channels, map_rect_to_surface};

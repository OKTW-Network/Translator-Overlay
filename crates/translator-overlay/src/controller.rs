//! Handle to the overlay window thread.

use std::thread::JoinHandle;

use tokio::sync::{mpsc, oneshot};
use tracing::{error, info};
use translator_core::OverlayConfig;

use crate::{
    command::{OverlayCommand, OverlayEvent},
    error::OverlayError,
    host::{OverlayHost, wake_overlay_thread},
};

/// Handle to a background overlay window thread.
pub struct OverlayController {
    tx: mpsc::UnboundedSender<OverlayCommand>,
    events: mpsc::UnboundedReceiver<OverlayEvent>,
    join: Option<JoinHandle<()>>,
}

impl OverlayController {
    /// Spawn the overlay host thread and create the layered window.
    pub async fn spawn(config: OverlayConfig) -> Result<Self, OverlayError> {
        let (tx, rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (ready_tx, ready_rx) = oneshot::channel();

        // HWND + WaitMessage are bound to this OS thread; not a Tokio task.
        let join = std::thread::Builder::new()
            .name("overlay".into())
            .spawn(move || match OverlayHost::create(config, event_tx) {
                Ok(mut host) => {
                    let _ = ready_tx.send(Ok(()));
                    host.run(rx);
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                }
            })
            .map_err(|e| OverlayError::Spawn(e.to_string()))?;

        match ready_rx.await {
            Ok(Ok(())) => {
                info!("overlay host ready");
                Ok(Self {
                    tx,
                    events: event_rx,
                    join: Some(join),
                })
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

    pub fn try_recv_event(&mut self) -> Option<OverlayEvent> {
        self.events.try_recv().ok()
    }

    pub async fn recv_event(&mut self) -> Option<OverlayEvent> {
        self.events.recv().await
    }

    /// Request shutdown and join the overlay thread (also safe if already stopped).
    pub async fn shutdown(&mut self) {
        let _ = self.send(OverlayCommand::Shutdown);
        if let Some(join) = self.join.take() {
            match tokio::task::spawn_blocking(move || join.join()).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => error!(?e, "overlay thread join failed"),
                Err(e) => error!(error = %e, "overlay thread join task failed"),
            }
        }
    }

    pub fn send(&self, cmd: OverlayCommand) -> Result<(), OverlayError> {
        self.tx.send(cmd).map_err(|_| OverlayError::NotRunning)?;
        wake_overlay_thread();
        Ok(())
    }
}

impl Drop for OverlayController {
    fn drop(&mut self) {
        let _ = self.send(OverlayCommand::Shutdown);
        // Do not join here — Drop may run on a Tokio worker. `shutdown().await` joins.
        let _ = self.join.take();
    }
}

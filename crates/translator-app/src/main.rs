//! Translator Overlay — WinUI 3 control app (windows-reactor).

mod pipeline;
mod taskbar_guard;
mod ui;

use std::sync::{Arc, OnceLock};

use parking_lot::RwLock;
use tracing::{error, info};
use translator_core::{AppConfig, AppState, config_path};
use windows_reactor::{App, Backdrop};

use crate::pipeline::{CmdTx, PipelineCommand, SharedState, spawn_pipeline};

/// Process-wide handles for the UI render function (set before App::render).
pub static APP_HANDLES: OnceLock<(SharedState, CmdTx)> = OnceLock::new();

#[tokio::main]
async fn main() {
    // Must run before any HWND is created (pipeline spawns the overlay window).
    // Otherwise GetClientRect / ClientToScreen stay in a mismatched DPI space
    // vs Graphics Capture physical pixels → overlay position/size drift.
    enable_per_monitor_dpi_v2();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Translator Overlay starting");

    let config = match config_path() {
        Ok(path) => {
            info!(path = %path.display(), "config path");
            AppConfig::load_or_create(&path).map_err(|e| e.to_string())
        }
        Err(e) => Err(e.to_string()),
    };
    let config = match config {
        Ok(c) => c,
        Err(e) => {
            error!("failed to load config: {e}");
            AppConfig::default()
        }
    };

    let state: SharedState = Arc::new(RwLock::new(AppState::new(config)));
    let (cmd_tx, pipeline) = spawn_pipeline(state.clone());

    APP_HANDLES.set((state.clone(), cmd_tx.clone())).expect("APP_HANDLES set once");

    // Framework-dependent: initialize Windows App Runtime via Bootstrap.dll.
    // Requires Windows App Runtime on the machine (install prompt if missing).
    if let Err(e) = windows_reactor::bootstrap() {
        error!(error = %e, "Windows App Runtime bootstrap failed");
        let _ = cmd_tx.send(PipelineCommand::Shutdown);
        let _ = pipeline.await;
        std::process::exit(1);
    }

    // WinUI / windows-reactor must pump on the OS main thread. `#[tokio::main]`
    // `block_on`s this future on that thread — do not move render off-thread.
    let result = App::new()
        .title("Translator Overlay")
        // Nav (168) + padding + Regions pane (480) + preview column.
        // WinUI multi-pane range is ~1100–1300 × 720–840.
        .inner_size(1280.0, 800.0)
        .backdrop(Backdrop::Mica)
        .render(crate::ui::app);

    let _ = cmd_tx.send(PipelineCommand::Shutdown);
    let _ = pipeline.await;

    if let Err(e) = result {
        error!(error = %e, "App::render failed");
        std::process::exit(1);
    }
}

/// Per-monitor DPI v2 so Win32 client rects match capture / overlay pixels.
fn enable_per_monitor_dpi_v2() {
    use windows::Win32::UI::HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext};
    // SAFETY: process-wide, no windows yet; failure is non-fatal (already set).
    let ok = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    if ok.is_err() {
        // Common when the host already set awareness; ignore.
    }
}

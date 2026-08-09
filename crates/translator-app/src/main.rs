//! Translator Overlay — WinUI 3 control app (windows-reactor).

mod pipeline;
mod ui;

use std::sync::{Arc, OnceLock};

use parking_lot::RwLock;
use tracing::{error, info};
use translator_core::{AppConfig, AppState};
use windows_reactor::{App, Backdrop};

use crate::pipeline::{CmdTx, PipelineCommand, SharedState, spawn_pipeline};

/// Process-wide handles for the UI render function (set before App::render).
pub static APP_HANDLES: OnceLock<(SharedState, CmdTx)> = OnceLock::new();

fn main() {
    // Must run before any HWND is created (pipeline spawns the overlay window).
    // Otherwise GetClientRect / ClientToScreen stay in a mismatched DPI space
    // vs Graphics Capture physical pixels → overlay position/size drift.
    enable_per_monitor_dpi_v2();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Translator Overlay starting");

    let config = match AppConfig::load_or_create_default() {
        Ok(c) => c,
        Err(e) => {
            error!("failed to load config: {e}");
            AppConfig::default()
        }
    };

    if let Ok(path) = translator_core::config_path() {
        info!(path = %path.display(), "config path");
    }

    let state: SharedState = Arc::new(RwLock::new(AppState::new(config)));
    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();
    let _pipeline = spawn_pipeline(Arc::clone(&state), cmd_rx);

    APP_HANDLES
        .set((Arc::clone(&state), cmd_tx.clone()))
        .expect("APP_HANDLES set once");

    // Framework-dependent: initialize Windows App Runtime via Bootstrap.dll.
    // Requires Windows App Runtime on the machine (install prompt if missing).
    if let Err(e) = windows_reactor::bootstrap() {
        error!(error = %e, "Windows App Runtime bootstrap failed");
        let _ = cmd_tx.send(PipelineCommand::Shutdown);
        std::process::exit(1);
    }

    let result = App::new()
        .title("Translator Overlay")
        .inner_size(960.0, 720.0)
        .backdrop(Backdrop::Mica)
        .render(ui::app);

    let _ = cmd_tx.send(PipelineCommand::Shutdown);

    if let Err(e) = result {
        error!(error = %e, "App::render failed");
        std::process::exit(1);
    }
}

/// Per-monitor DPI v2 so Win32 client rects match capture / overlay pixels.
fn enable_per_monitor_dpi_v2() {
    use windows::Win32::UI::HiDpi::{
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
    };
    // SAFETY: process-wide, no windows yet; failure is non-fatal (already set).
    let ok = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    if ok.is_err() {
        // Common when the host already set awareness; ignore.
    }
}

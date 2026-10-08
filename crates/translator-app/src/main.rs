//! Translator Overlay control app, built with WinUI 3 through windows-reactor.

mod attention;
mod pipeline;
mod taskbar_guard;
mod ui;

rust_i18n::i18n!("locales", fallback = "en");

use std::{
    env,
    os::windows::ffi::OsStrExt,
    process,
    sync::{Arc, OnceLock},
};

use parking_lot::RwLock;
use tracing::{error, info};
use translator_core::{AppConfig, AppState, UiLanguage, config_path};
use windows::{
    Win32::{
        System::LibraryLoader::{LoadLibraryW, SetDllDirectoryW},
        UI::HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext},
    },
    core::PCWSTR,
};
use windows_reactor::App;

use crate::{
    pipeline::{CmdTx, PipelineCommand, SharedState, spawn_pipeline},
    ui::AppRoot,
};

/// Process-wide handles for the UI, set before `App::run_component`.
pub static APP_HANDLES: OnceLock<(SharedState, CmdTx)> = OnceLock::new();

#[tokio::main]
async fn main() {
    load_onnxruntime();

    // This must run before any HWND exists, and the pipeline creates the overlay window.
    // Otherwise GetClientRect and ClientToScreen use a different DPI space than the
    // physical pixels from Graphics Capture, and the overlay drifts in position and size.
    // SAFETY: process-wide setting with no windows yet. It fails only when the host already
    // set an awareness, which is fine.
    let _ = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,ort::logging=warn")),
        )
        .init();

    info!("Translator Overlay starting");

    let config_file = match config_path() {
        Ok(path) => {
            info!(path = %path.display(), "config path");
            Some(path)
        }
        Err(e) => {
            error!("failed to resolve config path: {e}");
            None
        }
    };
    let mut config = match &config_file {
        Some(path) => match AppConfig::load_or_create(path) {
            Ok(c) => c,
            Err(e) => {
                error!("failed to load config: {e}");
                AppConfig::default()
            }
        },
        None => AppConfig::default(),
    };
    if config.ui.language.is_none() {
        config.ui.language = Some(UiLanguage::from_locale_name(&ui::system_locale_name()));
        if let Some(path) = &config_file
            && let Err(e) = config.save(path)
        {
            error!("failed to save initial ui language: {e}");
        }
    }
    ui::apply_ui_locale(config.ui.language);

    let state: SharedState = Arc::new(RwLock::new(AppState::new(config)));
    let (cmd_tx, pipeline) = spawn_pipeline(state.clone());

    APP_HANDLES.set((state.clone(), cmd_tx.clone())).expect("APP_HANDLES set once");

    // WinUI and windows-reactor must pump messages on the OS main thread. `#[tokio::main]`
    // runs this future with `block_on` on that thread, so do not move rendering elsewhere.
    // windows-reactor 0.100 bootstraps the WASDK framework itself, with no Bootstrap.dll or setup crate.
    let result = App::run_component::<AppRoot>(());

    let _ = cmd_tx.send(PipelineCommand::Shutdown);
    let _ = pipeline.await;

    if let Err(e) = result {
        error!(error = %e, "App::run_component failed");
        process::exit(1);
    }
}

/// Load our ONNX Runtime before WinUI maps the copy that ships with the Windows App Runtime.
fn load_onnxruntime() {
    let Some(dir) = env::current_exe().ok().and_then(|exe| {
        let dir = exe.parent()?;
        let lib = dir.join("lib");
        if lib.join("onnxruntime.dll").is_file() {
            Some(lib)
        } else if dir.join("onnxruntime.dll").is_file() {
            Some(dir.to_path_buf())
        } else {
            None
        }
    }) else {
        return;
    };
    let wide: Vec<u16> = dir.as_os_str().encode_wide().chain([0]).collect();
    let dll: Vec<u16> = dir.join("onnxruntime.dll").as_os_str().encode_wide().chain([0]).collect();
    // SAFETY: the paths are null-terminated. The LoadLibrary handle is leaked so ORT stays mapped.
    let _ = unsafe { SetDllDirectoryW(PCWSTR(wide.as_ptr())) };
    let _ = unsafe { LoadLibraryW(PCWSTR(dll.as_ptr())) };
}

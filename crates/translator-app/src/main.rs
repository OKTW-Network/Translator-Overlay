//! Translator Overlay — WinUI 3 control app.

#![windows_subsystem = "windows"]

mod attention;
mod hotkey;
mod pipeline;
mod taskbar_guard;
mod ui;

rust_i18n::i18n!("locales", fallback = "en");

use std::{
    env, fs,
    io::{self, Write},
    os::windows::ffi::OsStrExt,
    process,
    sync::{Arc, Mutex, OnceLock},
};

use parking_lot::RwLock;
use tracing::{error, info};
use translator_core::{AppConfig, AppState, UiLanguage, config_path, log_path, prev_log_path};
use windows::{
    Win32::{
        System::LibraryLoader::{LoadLibraryW, SetDllDirectoryW},
        UI::HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext},
    },
    core::PCWSTR,
};

use crate::pipeline::{CmdTx, PipelineCommand, SharedState, spawn_pipeline};

/// Process-wide handles for the UI (set before [`ui::run`]).
pub static APP_HANDLES: OnceLock<(SharedState, CmdTx)> = OnceLock::new();

#[tokio::main]
async fn main() {
    load_onnxruntime();

    // Must run before any HWND is created (pipeline spawns the overlay window).
    // Otherwise GetClientRect / ClientToScreen stay in a mismatched DPI space
    // vs Graphics Capture physical pixels → overlay position/size drift.
    // SAFETY: process-wide setting with no windows yet. It fails only when the host already
    // set an awareness, which is fine.
    let _ = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };

    init_tracing();

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
    hotkey::install(&state.read(), cmd_tx.clone());

    // WinUI must pump on the OS main thread. `#[tokio::main]` `block_on`s this future on
    // that thread — do not move the UI off-thread.
    let result = ui::run();

    hotkey::shutdown();
    let _ = cmd_tx.send(PipelineCommand::Shutdown);
    let _ = pipeline.await;

    if let Err(e) = result {
        error!(error = %e, "control window failed");
        process::exit(1);
    }
}

fn init_tracing() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,ort::logging=warn"));
    let file = log_path().ok().and_then(|current| {
        if let Ok(previous) = prev_log_path()
            && current.exists()
        {
            let _ = fs::remove_file(&previous);
            let _ = fs::rename(&current, &previous);
        }
        fs::File::create(current).ok()
    });
    let subscriber = tracing_subscriber::fmt().with_env_filter(env_filter).with_ansi(false);
    match file {
        Some(file) => subscriber.with_writer(Mutex::new(file)).init(),
        None => subscriber.init(),
    }
    std::panic::set_hook(Box::new(|info| {
        let mut msg = format!("panic: {info}\n");
        let bt = std::backtrace::Backtrace::capture();
        if bt.status() == std::backtrace::BacktraceStatus::Captured {
            msg.push_str(&format!("{bt}\n"));
        }
        if let Ok(path) = log_path()
            && let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path)
        {
            let _ = file.write_all(msg.as_bytes());
            let _ = file.flush();
        }
        let _ = io::stderr().write_all(msg.as_bytes());
    }));
}

/// Load our ORT before WinUI maps Windows App Runtime's copy.
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
    // SAFETY: null-terminated paths; LoadLibrary handle leaked so ORT stays mapped.
    let _ = unsafe { SetDllDirectoryW(PCWSTR(wide.as_ptr())) };
    let _ = unsafe { LoadLibraryW(PCWSTR(dll.as_ptr())) };
}

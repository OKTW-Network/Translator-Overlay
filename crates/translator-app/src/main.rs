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
        Foundation::HMODULE,
        System::{
            Diagnostics::Debug::{EXCEPTION_CONTINUE_SEARCH, EXCEPTION_POINTERS, SetUnhandledExceptionFilter},
            LibraryLoader::{
                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT, GetModuleFileNameW,
                GetModuleHandleExW, LoadLibraryW, SetDllDirectoryW,
            },
        },
    },
    core::PCWSTR,
};

use crate::pipeline::{CmdTx, PipelineCommand, SharedState, spawn_pipeline};

/// Process-wide handles for the UI (set before [`ui::run`]).
pub static APP_HANDLES: OnceLock<(SharedState, CmdTx)> = OnceLock::new();

#[tokio::main]
async fn main() {
    load_onnxruntime();
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
    // `xet` prefix-matches every hf-xet crate (`xet_client`, `xet_data`, …), which log each request at info.
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,ort::logging=warn,xet=warn"));
    let file = log_path().ok().and_then(|current| {
        if let Ok(previous) = prev_log_path()
            && current.exists()
        {
            let _ = fs::remove_file(&previous);
            let _ = fs::rename(&current, &previous);
        }
        // Append, like the panic hook's own handle: a plain write handle would later write over its message.
        fs::OpenOptions::new().create(true).append(true).open(current).ok()
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
        append_to_log(&msg);
        let _ = io::stderr().write_all(msg.as_bytes());
    }));
    // SAFETY: the filter is a function, valid for the whole process.
    unsafe { SetUnhandledExceptionFilter(Some(log_native_crash)) };
}

/// Write `msg` to the log file through its own handle: the tracing writer may be locked.
fn append_to_log(msg: &str) {
    if let Ok(path) = log_path()
        && let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path)
    {
        let _ = file.write_all(msg.as_bytes());
        let _ = file.flush();
    }
}

/// Log an unhandled native exception (an access violation inside a DLL, ...) as module + offset.
/// The panic hook never sees these, and Windows ends the process right after.
unsafe extern "system" fn log_native_crash(info: *const EXCEPTION_POINTERS) -> i32 {
    // SAFETY: Windows passes pointers that are valid for this call.
    let record = unsafe { &*(*info).ExceptionRecord };
    let address = record.ExceptionAddress;
    let mut module = HMODULE::default();
    let flags = GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT;
    // SAFETY: with FROM_ADDRESS the name is an address to look up, not a string; `module` is a valid out pointer.
    let found = unsafe { GetModuleHandleExW(flags, PCWSTR(address.cast()), &mut module) }.is_ok();
    let mut name = [0_u16; 260];
    let len = if found {
        // SAFETY: `module` is the loaded module found above; `name` is a writable buffer.
        unsafe { GetModuleFileNameW(Some(module), &mut name) as usize }
    } else {
        0
    };
    // No module: the offset is the absolute address.
    append_to_log(&format!(
        "crash: exception {:#010x} at {} + {:#x}\n",
        record.ExceptionCode.0,
        String::from_utf16_lossy(&name[..len]),
        address as usize - module.0 as usize
    ));
    EXCEPTION_CONTINUE_SEARCH
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

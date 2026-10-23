//! Adds the Windows App Runtime framework package to this unpackaged process.
//!
//! Adapted from windows-reactor 0.100.0 (`src/native/winui/bootstrap.rs`), <https://github.com/microsoft/windows-rs>.
//! Copyright (c) Microsoft Corporation. Licensed under MIT OR Apache-2.0.

use windows::{
    Win32::{
        Foundation::STATEREPOSITORY_E_DEPENDENCY_NOT_RESOLVED,
        Security::PSID,
        Storage::Packaging::Appx::{
            AddPackageDependency, AddPackageDependencyOptions_None, CreatePackageDependencyOptions_None, PACKAGE_VERSION,
            PACKAGE_VERSION_0, PACKAGEDEPENDENCY_CONTEXT, PackageDependencyLifetimeKind_Process,
            PackageDependencyProcessorArchitectures_Arm64, PackageDependencyProcessorArchitectures_Neutral,
            PackageDependencyProcessorArchitectures_X64, PackageDependencyProcessorArchitectures_X86, TryCreatePackageDependency,
        },
        System::Memory::{GetProcessHeap, HeapFree},
        UI::{
            Shell::ShellExecuteW,
            WindowsAndMessaging::{IDYES, MB_ICONERROR, MB_YESNO, MessageBoxW, SW_SHOWNORMAL},
        },
    },
    core::{HSTRING, PWSTR, w},
};

const FRAMEWORK_FAMILY: &str = "Microsoft.WindowsAppRuntime.2_8wekyb3d8bbwe";
/// Oldest runtime the bindings were generated from (2.4.0.0).
const MIN_VERSION: u64 = 0x0002_0004_0000_0000;

pub fn bootstrap() -> windows_core::Result<()> {
    let arch = if cfg!(target_arch = "aarch64") {
        PackageDependencyProcessorArchitectures_Arm64
    } else if cfg!(target_arch = "x86") {
        PackageDependencyProcessorArchitectures_X86
    } else {
        PackageDependencyProcessorArchitectures_X64
    };
    let version = PACKAGE_VERSION {
        Anonymous: PACKAGE_VERSION_0 { Version: MIN_VERSION },
    };
    // SAFETY: a null SID means the current user. Both returned strings come from the process heap.
    unsafe {
        let id = TryCreatePackageDependency(
            PSID::default(),
            &HSTRING::from(FRAMEWORK_FAMILY),
            version,
            arch | PackageDependencyProcessorArchitectures_Neutral,
            PackageDependencyLifetimeKind_Process,
            None,
            CreatePackageDependencyOptions_None,
        )
        .map_err(not_installed)?;
        let mut context = PACKAGEDEPENDENCY_CONTEXT::default();
        let mut full_name = PWSTR::null();
        let added = AddPackageDependency(id, 0, AddPackageDependencyOptions_None, &mut context, Some(&mut full_name));
        if let Ok(heap) = GetProcessHeap() {
            let _ = HeapFree(heap, Default::default(), Some(id.0.cast()));
            if !full_name.is_null() {
                let _ = HeapFree(heap, Default::default(), Some(full_name.0.cast()));
            }
        }
        added.map_err(not_installed)
    }
}

fn not_installed(error: windows::core::Error) -> windows_core::Error {
    if error.code() == STATEREPOSITORY_E_DEPENDENCY_NOT_RESOLVED {
        let text =
            HSTRING::from("You must install Windows App Runtime 2.4 (x64) to run this application.\n\nDo you want to download it now?");
        // SAFETY: modal message box and shell open with valid strings.
        unsafe {
            if MessageBoxW(None, &text, w!("Translator Overlay"), MB_YESNO | MB_ICONERROR) == IDYES {
                ShellExecuteW(
                    None,
                    w!("open"),
                    w!("https://learn.microsoft.com/windows/apps/windows-app-sdk/downloads"),
                    None,
                    None,
                    SW_SHOWNORMAL,
                );
            }
        }
    }
    windows_core::Error::new(windows_core::HRESULT(error.code().0), error.message())
}

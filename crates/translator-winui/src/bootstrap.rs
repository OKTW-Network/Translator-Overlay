//! Adds the Windows App Runtime framework package to this unpackaged process.
//!
//! Adapted from windows-reactor 0.100.0 (`src/native/winui/bootstrap.rs`), <https://github.com/microsoft/windows-rs>.
//! Copyright (c) Microsoft Corporation. Licensed under MIT OR Apache-2.0.
//!
//! The Windows 10 path follows the Windows App SDK bootstrapper (`dev/WindowsAppRuntime_BootstrapDLL/MddBootstrap.cpp`),
//! <https://github.com/microsoft/WindowsAppSDK>. Copyright (c) Microsoft Corporation. Licensed under MIT.

use windows::{
    Win32::{
        Foundation::STATEREPOSITORY_E_DEPENDENCY_NOT_RESOLVED,
        Security::PSID,
        Storage::Packaging::Appx::{
            AddPackageDependencyOptions, AddPackageDependencyOptions_None, CreatePackageDependencyOptions,
            CreatePackageDependencyOptions_None, GetPackagePathByFullName, GetPackagesByPackageFamily, PACKAGE_VERSION, PACKAGE_VERSION_0,
            PACKAGEDEPENDENCY_CONTEXT, PackageDependencyLifetimeKind, PackageDependencyLifetimeKind_Process,
            PackageDependencyProcessorArchitectures, PackageDependencyProcessorArchitectures_Arm64,
            PackageDependencyProcessorArchitectures_Neutral, PackageDependencyProcessorArchitectures_X64,
            PackageDependencyProcessorArchitectures_X86,
        },
        System::{
            LibraryLoader::{GetModuleHandleW, GetProcAddress, LOAD_WITH_ALTERED_SEARCH_PATH, LoadLibraryExW},
            Memory::{GetProcessHeap, HeapFree},
        },
        UI::{
            Shell::ShellExecuteW,
            WindowsAndMessaging::{IDYES, MB_ICONERROR, MB_YESNO, MessageBoxW, SW_SHOWNORMAL},
        },
    },
    core::{HRESULT, HSTRING, PCWSTR, PWSTR, s, w},
};

const FRAMEWORK_FAMILY: &str = "Microsoft.WindowsAppRuntime.2_8wekyb3d8bbwe";
/// Oldest runtime the bindings were generated from (2.4.0.0).
const MIN_VERSION: u64 = 0x0002_0004_0000_0000;

/// `TryCreatePackageDependency` (appmodel.h); `MddTryCreatePackageDependency` has the same signature.
type TryCreate = unsafe extern "system" fn(
    PSID,
    PCWSTR,
    PACKAGE_VERSION,
    PackageDependencyProcessorArchitectures,
    PackageDependencyLifetimeKind,
    PCWSTR,
    CreatePackageDependencyOptions,
    *mut PWSTR,
) -> HRESULT;
/// `AddPackageDependency` (appmodel.h); `MddAddPackageDependency` has the same signature.
type Add = unsafe extern "system" fn(PCWSTR, i32, AddPackageDependencyOptions, *mut PACKAGEDEPENDENCY_CONTEXT, *mut PWSTR) -> HRESULT;
/// What `GetProcAddress` returns.
type Proc = unsafe extern "system" fn() -> isize;

pub fn bootstrap() -> windows_core::Result<()> {
    let (try_create, add) = match os_api() {
        Some(api) => api,
        None => framework_api().map_err(not_installed)?,
    };
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
    let family = HSTRING::from(FRAMEWORK_FAMILY);
    let mut id = PWSTR::null();
    // SAFETY: a null SID means the current user; `family` outlives the call; `id` receives a process-heap string.
    unsafe {
        try_create(
            PSID::default(),
            PCWSTR(family.as_ptr()),
            version,
            arch | PackageDependencyProcessorArchitectures_Neutral,
            PackageDependencyLifetimeKind_Process,
            PCWSTR::null(),
            CreatePackageDependencyOptions_None,
            &mut id,
        )
    }
    .ok()
    .map_err(not_installed)?;
    let mut context = PACKAGEDEPENDENCY_CONTEXT::default();
    let mut full_name = PWSTR::null();
    // SAFETY: `id` is the dependency just created; `full_name` receives a process-heap string.
    let added = unsafe { add(PCWSTR(id.0), 0, AddPackageDependencyOptions_None, &mut context, &mut full_name) }.ok();
    // SAFETY: no preconditions.
    if let Ok(heap) = unsafe { GetProcessHeap() } {
        // SAFETY: `id` was allocated on the process heap and is not used again.
        let _ = unsafe { HeapFree(heap, Default::default(), Some(id.0.cast())) };
        if !full_name.is_null() {
            // SAFETY: `full_name` was allocated on the process heap and is not used again.
            let _ = unsafe { HeapFree(heap, Default::default(), Some(full_name.0.cast())) };
        }
    }
    added.map_err(not_installed)
}

/// Windows 11's dynamic dependency API. Looked up at run time: a static import keeps the exe from loading on Windows 10.
fn os_api() -> Option<(TryCreate, Add)> {
    // SAFETY: null-terminated name; kernelbase is mapped into every process.
    let kernelbase = unsafe { GetModuleHandleW(w!("kernelbase.dll")) }.ok()?;
    // SAFETY: valid module and null-terminated name.
    let try_create = unsafe { GetProcAddress(kernelbase, s!("TryCreatePackageDependency")) }?;
    // SAFETY: valid module and null-terminated name.
    let add = unsafe { GetProcAddress(kernelbase, s!("AddPackageDependency")) }?;
    // SAFETY: both exports have exactly these signatures.
    Some(unsafe { (core::mem::transmute::<Proc, TryCreate>(try_create), core::mem::transmute::<Proc, Add>(add)) })
}

/// Windows 10: the framework package's own implementation of the same API, as the official bootstrapper uses.
// shortcut: no DDLM, so a runtime update while the app runs on Windows 10 may remove the version in use; port the DDLM
// activation from MddBootstrap.cpp if that gets reported.
fn framework_api() -> windows::core::Result<(TryCreate, Add)> {
    let dir = framework_dir().ok_or_else(|| windows::core::Error::from(STATEREPOSITORY_E_DEPENDENCY_NOT_RESOLVED))?;
    let dll = HSTRING::from(dir + "\\Microsoft.WindowsAppRuntime.dll");
    // SAFETY: null-terminated full path; the module stays loaded for the life of the process.
    let module = unsafe { LoadLibraryExW(&dll, None, LOAD_WITH_ALTERED_SEARCH_PATH) }?;
    // SAFETY: valid module and null-terminated name.
    let try_create =
        unsafe { GetProcAddress(module, s!("MddTryCreatePackageDependency")) }.ok_or_else(windows::core::Error::from_thread)?;
    // SAFETY: valid module and null-terminated name.
    let add = unsafe { GetProcAddress(module, s!("MddAddPackageDependency")) }.ok_or_else(windows::core::Error::from_thread)?;
    // SAFETY: both exports have exactly these signatures (MsixDynamicDependency.h).
    Ok(unsafe { (core::mem::transmute::<Proc, TryCreate>(try_create), core::mem::transmute::<Proc, Add>(add)) })
}

/// Install folder of the newest runtime for this architecture at or above `MIN_VERSION`.
fn framework_dir() -> Option<String> {
    let family = HSTRING::from(FRAMEWORK_FAMILY);
    let (mut count, mut len) = (0, 0);
    // SAFETY: size query only.
    let _ = unsafe { GetPackagesByPackageFamily(&family, &mut count, None, &mut len, None) };
    if count == 0 {
        return None;
    }
    let mut names = vec![PWSTR::null(); count as usize];
    let mut buffer = vec![0u16; len as usize];
    // SAFETY: `names` and `buffer` have exactly the sizes reported above.
    unsafe { GetPackagesByPackageFamily(&family, &mut count, Some(names.as_mut_ptr()), &mut len, Some(PWSTR(buffer.as_mut_ptr()))) }
        .ok()
        .ok()?;
    let arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else if cfg!(target_arch = "x86") {
        "x86"
    } else {
        "x64"
    };
    let (_, name) = names
        .iter()
        // SAFETY: each entry points into `buffer`, null-terminated.
        .filter_map(|name| unsafe { name.to_string() }.ok())
        .filter_map(|name| {
            // Full names are `Name_Version_Arch_ResourceId_PublisherId`.
            let mut fields = name.split('_').skip(1);
            let version = fields
                .next()?
                .split('.')
                .try_fold(0u64, |v, part| Some(v << 16 | u64::from(part.parse::<u16>().ok()?)))?;
            (fields.next()? == arch && version >= MIN_VERSION).then_some((version, name))
        })
        .max()?;
    let name = HSTRING::from(name);
    let mut len = 0;
    // SAFETY: size query only.
    let _ = unsafe { GetPackagePathByFullName(&name, &mut len, None) };
    let mut path = vec![0u16; len as usize];
    // SAFETY: `path` has exactly the size reported above.
    unsafe { GetPackagePathByFullName(&name, &mut len, Some(PWSTR(path.as_mut_ptr()))) }
        .ok()
        .ok()?;
    Some(String::from_utf16_lossy(&path[..len as usize - 1]))
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

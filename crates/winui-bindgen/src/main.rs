//! Regenerates the `translator-winui` bindings from the metadata of the installed Windows App Runtime 2.
//!
//! Run with `cargo run -p winui-bindgen` after editing a filter file.

use std::path::{Path, PathBuf};

use windows::{
    Win32::Storage::Packaging::Appx::{GetPackagePathByFullName, GetPackagesByPackageFamily},
    core::{HSTRING, PCWSTR, PWSTR},
};

/// Same family the app binds to at startup (`translator-winui` bootstrap).
const FAMILY: &str = "Microsoft.WindowsAppRuntime.2_8wekyb3d8bbwe";

/// The files covering every namespace the filters pull from, plus the ones they reference.
const METADATA: [&str; 6] = [
    "Microsoft.Foundation.winmd",
    "Microsoft.Graphics.winmd",
    "Microsoft.UI.Text.winmd",
    "Microsoft.UI.winmd",
    "Microsoft.UI.Xaml.winmd",
    "Microsoft.Windows.ApplicationModel.Resources.winmd",
];

/// Oldest version the app accepts at startup (`translator-winui` bootstrap).
const MIN_VERSION: [u32; 2] = [2, 4];

/// Install folder of the oldest installed runtime the app accepts, so the bindings never get ahead of it.
fn runtime_dir() -> PathBuf {
    let family = HSTRING::from(FAMILY);
    let (mut count, mut len) = (0, 0);
    // SAFETY: the first calls only report sizes; the second ones fill buffers of exactly those sizes.
    unsafe {
        let _ = GetPackagesByPackageFamily(&family, &mut count, None, &mut len, None);
        assert!(count > 0, "{FAMILY} is not installed (Windows App Runtime 2)");
        let mut names = vec![PWSTR::null(); count as usize];
        let mut buffer = vec![0u16; len as usize];
        GetPackagesByPackageFamily(&family, &mut count, Some(names.as_mut_ptr()), &mut len, Some(PWSTR(buffer.as_mut_ptr())))
            .ok()
            .expect("list runtime packages");
        // Full names are `Name_Version_Arch_ResourceId_PublisherId`; every architecture ships the same metadata.
        let version = |name: &String| -> Vec<u32> {
            let field = name.split('_').nth(1).unwrap_or_default();
            field.split('.').map(|part| part.parse().unwrap_or(0)).collect()
        };
        let name = names
            .iter()
            .map(|name| name.to_string().expect("package full name"))
            .filter(|name| version(name).as_slice() >= MIN_VERSION.as_slice())
            .min_by_key(version)
            .expect("Windows App Runtime 2.4 or later is not installed");
        let name = HSTRING::from(name);
        let mut len = 0;
        let _ = GetPackagePathByFullName(PCWSTR(name.as_ptr()), &mut len, None);
        let mut path = vec![0u16; len as usize];
        GetPackagePathByFullName(PCWSTR(name.as_ptr()), &mut len, Some(PWSTR(path.as_mut_ptr())))
            .ok()
            .expect("runtime package path");
        PathBuf::from(String::from_utf16_lossy(&path[..len as usize - 1]))
    }
}

fn main() {
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = here.join("../translator-winui/src/generated");
    let runtime = runtime_dir();
    println!("metadata: {}", runtime.display());
    let inputs: Vec<PathBuf> = METADATA.iter().map(|file| runtime.join(file)).collect();
    windows_bindgen::builder()
        .inputs(&inputs)
        .input_default()
        .output(out.join("bindings.rs"))
        .flat()
        .filter_file(here.join("filter.txt"))
        .write();
    // `compose` only works in minimal mode, so the Application subclass gets its own module.
    // Options adapted from windows-reactor's binding generator (windows-rs `crates/tools/reactor/src/main.rs`,
    // https://github.com/microsoft/windows-rs, Copyright (c) Microsoft Corporation, MIT OR Apache-2.0).
    windows_bindgen::builder()
        .inputs(&inputs)
        .input_default()
        .output(out.join("app_bindings.rs"))
        .implements([
            "Microsoft.UI.Xaml.IApplicationOverrides",
            "Microsoft.UI.Xaml.Markup.IXamlMetadataProvider",
        ])
        .compose("Microsoft.UI.Xaml.Application")
        .minimal()
        .flat()
        .filter_file(here.join("app_filter.txt"))
        .write();
}

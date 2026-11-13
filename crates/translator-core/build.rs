use std::{env, fs, path::PathBuf};

// shortcut: ort-sys rc.13's `copy-dylibs` panics on the Rust 1.100 build-dir layout, so `disable-linking`
// turns its build script off and this one links and stages ONNX Runtime; drop once oar-ocr allows ort >= rc.14.
fn main() {
    println!("cargo:rerun-if-env-changed=ORT_LIB_PATH");
    let lib_dir = PathBuf::from(env::var_os("ORT_LIB_PATH").expect("ORT_LIB_PATH is set in .cargo/config.toml"));
    println!("cargo:rustc-link-lib=onnxruntime");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    // Windows loads the DLLs from next to the executable; `cargo run` / `cargo test` also put this directory on PATH.
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    let profile_dir = out_dir
        .ancestors()
        .find(|dir| dir.file_name().is_some_and(|name| name == "build"))
        .and_then(|build| build.parent())
        .expect("OUT_DIR is under <target>/<profile>/build");
    for name in ["onnxruntime.dll", "DirectML.dll", "webgpu_dawn.dll", "dxcompiler.dll"] {
        let src = lib_dir.join(name);
        println!("cargo:rerun-if-changed={}", src.display());
        fs::copy(&src, profile_dir.join(name))
            .unwrap_or_else(|e| panic!("copy {}: {e} (run scripts/build-onnxruntime.ps1 first)", src.display()));
    }
}

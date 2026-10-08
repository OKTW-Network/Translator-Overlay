use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rustc-link-arg=/DELAYLOAD:onnxruntime.dll");
    println!("cargo:rustc-link-lib=delayimp");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("FileDescription", "Translator Overlay");
        res.set("ProductName", "Translator Overlay");
        res.set("LegalCopyright", "Unlicense");
        res.compile().expect("embed Windows resources");
    }

    let src = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("assets/icon.ico");
    if let Ok(out) = env::var("OUT_DIR")
        && let Some(profile_dir) = Path::new(&out).ancestors().nth(3)
        && let Err(e) = fs::copy(&src, profile_dir.join("icon.ico"))
    {
        println!("cargo:warning=copy icon.ico: {e}");
    }
}

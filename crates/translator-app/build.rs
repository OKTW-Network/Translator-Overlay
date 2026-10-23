use std::env;

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=locales");
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
}

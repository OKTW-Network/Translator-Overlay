fn main() {
    println!("cargo:rustc-link-arg=/DELAYLOAD:onnxruntime.dll");
    println!("cargo:rustc-link-lib=delayimp");

    // Explorer / taskbar / Alt+Tab icon. Regenerate app.ico with scripts/icon-gen after editing the SVGs.
    println!("cargo:rerun-if-changed=assets/icon/app.ico");
    winresource::WindowsResource::new()
        .set_icon("assets/icon/app.ico")
        .set("ProductName", "Translator Overlay")
        .set("FileDescription", "Translator Overlay")
        .compile()
        .expect("embed Windows resources");
}

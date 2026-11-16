fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    // The built-in module loads llama.cpp's DLLs itself, so this crate's tests start without them.
    // translator-app's build.rs does the same for the app.
    println!("cargo:rustc-link-arg=/DELAYLOAD:llama.dll");
    println!("cargo:rustc-link-arg=/DELAYLOAD:ggml.dll");
    println!("cargo:rustc-link-lib=delayimp");
}

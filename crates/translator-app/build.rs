fn main() {
    println!("cargo:rustc-link-arg=/DELAYLOAD:onnxruntime.dll");
    println!("cargo:rustc-link-lib=delayimp");
}

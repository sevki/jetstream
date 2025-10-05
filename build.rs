fn main() {
    // Check if the target is WebAssembly
    let target = std::env::var("TARGET").unwrap_or_default();

    if target.contains("wasm32") {
        println!("cargo:rustc-cfg=wasm");
    }
}

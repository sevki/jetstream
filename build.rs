use cfg_aliases::cfg_aliases;

fn main() {
    // Check if the target is WebAssembly
    let target = std::env::var("TARGET").unwrap_or_default();

    if target.contains("wasm32") {
        println!("cargo:rustc-cfg=wasm");
    }

    cfg_aliases! {
        wasm_browser: { all(target_family = "wasm", target_os = "unknown") },
        cloudflare: { all(target_arch = "wasm32", target_os = "unknown", feature = "worker") },
        unix_with_tokio: { all(unix, feature = "tokio") },
    }
}

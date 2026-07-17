use cfg_aliases::cfg_aliases;

fn main() {
    // Set tag_pool_backend from env var, defaulting to "notify"
    let backend = std::env::var("JETSTREAM_TAG_POOL_BACKEND")
        .unwrap_or_else(|_| "notify".to_string());

    // Emit the cfg for the selected backend
    match backend.as_str() {
        "channel" => println!("cargo::rustc-cfg=channel"),
        "notify" => println!("cargo::rustc-cfg=notify"),
        "semaphor" => println!("cargo::rustc-cfg=semaphor"),
        _ => panic!(
            "Invalid JETSTREAM_TAG_POOL_BACKEND: {}. Must be channel, notify, or semaphor",
            backend
        ),
    }

    // Register check-cfg for the tag pool backends
    println!("cargo::rustc-check-cfg=cfg(channel)");
    println!("cargo::rustc-check-cfg=cfg(notify)");
    println!("cargo::rustc-check-cfg=cfg(semaphor)");

    cfg_aliases! {
        native: { not(target_arch = "wasm32") },
        tokio_unix: { all(any(target_os = "linux", target_os = "macos")) },
        cloudflare: { all(target_arch = "wasm32", target_os = "unknown", feature = "worker") },
    }
}

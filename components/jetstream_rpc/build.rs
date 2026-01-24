use cfg_aliases::cfg_aliases;

fn main() {
    cfg_aliases! {
        native: { not(target_arch = "wasm32") },
        tokio_unix: { all(any(target_os = "linux", target_os = "macos")) },
        cloudflare: { all(target_arch = "wasm32", target_os = "unknown", feature = "worker") },
    }
}

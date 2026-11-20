//! Gates the `nightly` feature's generic-const-args code behind a `jetstream_gca` cfg.
//!
//! The feature alone isn't enough: `#![feature(gca_const_items)]` is rejected
//! on stable and beta, and needs `-Znext-solver` on nightly. `--all-features`
//! (which CI uses) turns `nightly` on everywhere, so the cfg is only set when
//! the compiler can actually build it.
use std::{env, process::Command};

fn main() {
    println!("cargo::rustc-check-cfg=cfg(jetstream_gca)");
    println!("cargo::rerun-if-env-changed=CARGO_ENCODED_RUSTFLAGS");

    if env::var_os("CARGO_FEATURE_NIGHTLY").is_none() {
        return;
    }

    let rustc = env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let is_nightly = Command::new(rustc)
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .is_some_and(|v| v.contains("nightly"));
    let next_solver = env::var("CARGO_ENCODED_RUSTFLAGS")
        .is_ok_and(|f| f.contains("-Znext-solver"));

    if is_nightly && next_solver {
        println!("cargo::rustc-cfg=jetstream_gca");
    } else {
        println!(
            "cargo::warning=feature `nightly` is a no-op without a nightly compiler and RUSTFLAGS=\"-Znext-solver\""
        );
    }
}

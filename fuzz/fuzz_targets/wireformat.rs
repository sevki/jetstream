#![no_main]

use libfuzzer_sys::fuzz_target;

// The body lives under `tests/` so that it survives `cargo package`:
// `fuzz/` is a nested package and is excluded from the archive, while
// `tests/fuzz_regressions.rs` — which replays this target's findings —
// is not. See `tests/fuzz_body/mod.rs`.
#[path = "../../tests/fuzz_body/mod.rs"]
mod body;

fuzz_target!(|data: &[u8]| {
    body::exercise(data);
});

//! Replay every input the fuzzer has ever found interesting.
//!
//! A fuzzer's value is cumulative: the corpus is what it has learned
//! about the input space, and a crasher is a bug someone fixed. Both
//! were being thrown away — `fuzz/corpus` and `fuzz/artifacts` were
//! gitignored, so the nightly run started from nothing every midnight
//! and any input that broke something died with the runner.
//!
//! They are committed now, which only means something if they are *run*.
//! This runs them, on every pull request, through the same code the fuzz
//! target exercises — no nightly toolchain, no `cargo-fuzz`, no six-hour
//! job. A crasher that has been fixed stays fixed, and the corpus that
//! took hours to grow is a regression suite that takes milliseconds.

#![cfg(feature = "9p")]

use std::{fs, path::Path};

// The same code the fuzz target runs, shared rather than copied: a
// replay that has drifted from the target it replays is worse than none,
// because it goes green while the thing it guards has moved. The fuzz
// target includes this same file by path.
mod fuzz_body;
use fuzz_body as body;

// The same again for the scenario target: one runner, one oracle, two
// callers. `turmoil_scenarios.rs` mounts this module too.
#[path = "scenario_body/mod.rs"]
mod scenario_body;

/// Every file directly inside `dir`, ignoring the housekeeping ones.
fn inputs(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<(String, Vec<u8>)> = entries
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if !path.is_file() {
                return None;
            }
            let name = path.file_name()?.to_string_lossy().into_owned();
            if name.starts_with('.') {
                return None;
            }
            Some((name, fs::read(&path).ok()?))
        })
        .collect();
    // Deterministic order, so a failure names the same input every run.
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

/// Panicking on a committed input is the failure. `exercise` is expected
/// to reject nonsense — that is what decoding untrusted bytes means —
/// but never to come apart on it.
#[test]
fn every_committed_fuzz_input_still_survives() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let corpus = inputs(&root.join("fuzz/corpus/wireformat"));
    let crashers = inputs(&root.join("fuzz/artifacts/wireformat"));

    for (name, data) in corpus.iter().chain(crashers.iter()) {
        // Caught and re-raised with the file name attached, because "a
        // fuzz input panicked" is not something anyone can act on: the
        // whole value of a committed crasher is knowing *which* bytes.
        let survived = std::panic::catch_unwind(|| body::exercise(data));
        assert!(
            survived.is_ok(),
            "fuzz input `{name}` panicked; it is committed precisely so \
             this cannot regress silently",
        );
    }

    eprintln!(
        "replayed {} corpus inputs and {} crashers",
        corpus.len(),
        crashers.len(),
    );
}

/// The same for the `scenarios` target.
///
/// Separate from the wireformat replay because the cost is different by
/// three orders of magnitude: one wireformat input is a decode, one
/// scenario input is a whole simulated session at roughly 60ms. That is
/// affordable while this corpus is small, and it is *supposed* to be
/// small — the corpus that matters for a scenario is the minimised one.
/// If this test ever becomes the slow part of a pull request, the answer
/// is `cargo fuzz cmin scenarios`, not a partial replay: replaying some
/// of the corpus would go green while a regression sat in the rest.
#[test]
fn every_committed_scenario_still_holds() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let corpus = inputs(&root.join("fuzz/corpus/scenarios"));
    let crashers = inputs(&root.join("fuzz/artifacts/scenarios"));

    for (name, data) in corpus.iter().chain(crashers.iter()) {
        // A committed input that no longer decodes to a scenario is a
        // silent hole in the corpus, not a pass: the mapping from bytes
        // to actions changed and this input now exercises nothing.
        let scenario = scenario_body::Scenario::from_bytes(data)
            .unwrap_or_else(|| {
                panic!(
                    "committed scenario input `{name}` no longer describes \
                     a scenario; the byte mapping changed underneath it",
                )
            });
        let seed = scenario.seed;
        if let Err(e) = scenario_body::run_scenario(scenario) {
            panic!("committed scenario `{name}` (seed {seed}) failed: {e}");
        }
    }

    eprintln!(
        "replayed {} scenario corpus inputs and {} crashers",
        corpus.len(),
        crashers.len(),
    );
}

/// The corpus directories must exist, so the nightly run has somewhere
/// to accumulate into and this replay has somewhere to look.
///
/// Without this, deleting them is a silent no-op: the replay above finds
/// nothing, passes, and reports that it guarded a suite that is not
/// there.
///
/// The check is anchored on `fuzz/` itself rather than asserted flatly,
/// because there is one place the directories are legitimately absent:
/// `fuzz/` is a nested package, so `cargo package` drops the whole tree
/// from the published archive. Asserting flatly would fail the published
/// crate's suite over an absence nobody consuming it can fix. Where
/// `fuzz/` exists — the repository, which is the only place the nightly
/// run and this replay both happen — the corpus must be there with it.
#[test]
fn the_corpus_directories_are_present() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    if !root.join("fuzz").is_dir() {
        eprintln!("no `fuzz/` tree: packaged crate, nothing to guard");
        return;
    }
    for dir in [
        "fuzz/corpus/wireformat",
        "fuzz/artifacts/wireformat",
        "fuzz/corpus/scenarios",
        "fuzz/artifacts/scenarios",
    ] {
        assert!(
            root.join(dir).is_dir(),
            "{dir} is missing: the nightly fuzzer has nowhere to keep \
             what it finds, and the replay has nothing to guard",
        );
    }
}

#![no_main]

use libfuzzer_sys::fuzz_target;

// The scenario runner and its invariants, shared with
// `tests/turmoil_scenarios.rs` rather than copied. The seed walk there
// and this target must explore the same space against the same oracle:
// if they drifted, a seed filed by one would not reproduce under the
// other. Under `tests/` for the same reason as `fuzz_body`: `fuzz/` is a
// nested package and `cargo package` drops it wholesale.
#[path = "../../tests/scenario_body/mod.rs"]
mod body;

// Coverage-guided search over the session layer, not just the decoder.
//
// The `wireformat` target fuzzes one function: bytes in, frame out. The
// invariants worth the most here are the ones about *sequences* — that a
// producer stops when its subscriber goes, that nothing follows an
// ending — and those need a whole session, a server, and a network that
// can be cut. Turmoil supplies all three deterministically, so a
// scenario is a pure function of its bytes and a crasher replays
// exactly.
fuzz_target!(|data: &[u8]| {
    let Some(scenario) = body::Scenario::from_bytes(data) else {
        return;
    };
    // The network seed, so a panic message names the thing that would
    // let someone reproduce it outside the fuzzer.
    let seed = scenario.seed;

    // Turmoil catches a panicking host and hands it back as an error, so
    // an invariant that fails inside the simulation does not reach
    // libfuzzer on its own — it has to be re-raised here or every
    // finding would be silently swallowed and the target would report
    // that it never found anything. Every error is a finding: the runner
    // already tolerates the outcomes that are the scenario's doing
    // rather than the code's.
    if let Err(e) = body::run_scenario(scenario) {
        panic!("scenario failed (network seed {seed}): {e}");
    }
});

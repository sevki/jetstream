//! The nightly scenario search, and the replay of every seed it found.
//!
//! The scenarios themselves — the actions, the invariants, the simulated
//! network they run on — live in `tests/scenario_body/mod.rs`. They are
//! shared rather than duplicated because the `scenarios` fuzz target
//! runs the *same* runner against the *same* oracle: a fuzzer and a
//! replay that have drifted apart are worse than either alone, since
//! both go green while the thing they guard has moved underneath them.
//! `tests/fuzz_body/mod.rs` is split out for exactly the same reason.
//!
//! Two searches reach that runner. This one walks seeds in order, which
//! is cheap, reproducible and completely blind. The fuzz target steers
//! by coverage instead, keeping the byte strings that reached new code.
//! Neither subsumes the other: a seed is a one-number bug report anyone
//! can re-run, and coverage is how you find the scenario nobody would
//! have counted up to.

#[path = "scenario_body/mod.rs"]
mod body;

use body::{run_scenario, Scenario};

/// The seeds that have failed before, from `tests/seeds/scenarios.txt`.
fn committed_seeds() -> Vec<u64> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/seeds/scenarios.txt");
    let Ok(text) = std::fs::read_to_string(&path) else {
        panic!(
            "{} is missing: the regression corpus has to exist for \
                this test to be replaying anything",
            path.display()
        );
    };
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            line.parse().unwrap_or_else(|_| {
                panic!("`{line}` in scenarios.txt is not a u64 seed")
            })
        })
        .collect()
}

/// Read a `u64` from the environment, or fall back.
///
/// A malformed value falls back silently on purpose: this runs inside a
/// test, and a nightly job that dies on a typo in its own workflow finds
/// nothing at all.
fn env_u64(name: &str, fallback: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(fallback)
}

/// r[verify jetstream.subscription.overview]
/// The search itself. Each seed is an independent story; a failure names
/// the seed, and `run_scenario(Scenario::from_seed(n))` replays it
/// exactly.
#[test]
fn scenarios_hold_their_invariants() {
    // Deeper on demand: `JETSTREAM_SCENARIO_SEEDS=100000 cargo test
    // --test turmoil_scenarios` searches as long as you like, with the
    // same runner and the same oracle. The default is what CI can
    // afford on every push.
    let seeds = env_u64("JETSTREAM_SCENARIO_SEEDS", 256);

    // Where the window starts. This exists because a search that always
    // starts at zero is not a search: the nightly job ran 20000 seeds
    // every night and they were the same 20000 seeds, so after the first
    // night it could not find anything it had not already found. It was
    // an expensive way to re-run a fixed test suite.
    //
    // The nightly passes a start that moves — see `fuzzing.yml`, which
    // derives it from the run number so consecutive nights cover
    // adjacent, non-overlapping windows. Zero here keeps the default a
    // fixed, deterministic band, which is what a pull request wants: the
    // same 256 scenarios on every push, so a red result means the change
    // did it.
    let start = env_u64("JETSTREAM_SCENARIO_START", 0);
    let end = start.saturating_add(seeds);

    // Every seed that has ever failed, replayed first: a fixed bug
    // stays fixed, and one that is not yet fixed keeps this red until it
    // is. The sweep below searches for new ones; this makes sure the old
    // ones cannot come back unnoticed.
    let mut exercised = 0usize;
    for seed in committed_seeds() {
        exercised += run_scenario(Scenario::from_seed(seed))
            .unwrap_or_else(|e| panic!("committed seed {seed} regressed: {e}"));
    }

    // Named in the log so a nightly failure says which window found it,
    // and so a green night is visibly a *different* green night from the
    // one before it.
    println!("searching scenario seeds {start}..{end}");
    for seed in start..end {
        let scenario = Scenario::from_seed(seed);
        let items = run_scenario(scenario)
            .unwrap_or_else(|e| panic!("seed {seed} failed: {e}"));
        exercised += items;
    }
    // A run that delivered nothing would pass every assertion above
    // while testing none of them.
    //
    // Proportional rather than a fixed count. The old floor was a bare
    // `> 50`, which silently assumed the window was always the default
    // 256 seeds — the moment the window became configurable that number
    // meant "fail on any small window" and nothing else. A rate says
    // what is actually being claimed: scenarios deliver items, so a
    // window that delivered almost none did not run.
    //
    // Loose on purpose. The observed rate is a few items per seed; a
    // quarter of one per seed is far enough below that to never fire on
    // an unlucky window, and far enough above zero to catch a runner
    // that has stopped delivering.
    let floor = (end - start) / 4;
    println!("searched seeds {start}..{end}: {exercised} items delivered");
    assert!(
        exercised as u64 > floor,
        "the scenarios must actually exercise delivery; saw {exercised} \
         items over seeds {start}..{end}, wanted more than {floor}",
    );
}

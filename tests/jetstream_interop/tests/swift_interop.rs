// r[verify jetstream.interop.swift]
// r[verify jetstream.interop.roundtrip]
// r[verify jetstream.interop.byte-identical]
// r[verify jetstream.interop.proptest]
// r[verify jetstream.interop.protocol]

use jetstream_interop::*;
use proptest::prelude::*;
use std::process::{Command, Stdio};

/// Find the repository root relative to CARGO_MANIFEST_DIR.
fn repo_root() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../..")
}

/// Check if a command is available on PATH.
fn command_exists(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

/// Spawn the Swift interop helper.
///
/// Tries in order:
/// 1. `swift run` directly (works in CI and when Swift is installed)
/// 2. `podman run swift:latest` (fallback for dev machines without Swift)
///
/// Returns `None` if neither `swift` nor `podman` is available.
fn spawn_swift_helper() -> Option<std::process::Child> {
    let root = repo_root();

    if command_exists("swift") {
        return Some(
            Command::new("swift")
                .args(["run", "JetStreamInteropHelper"])
                .current_dir(&root)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()
                .expect("failed to spawn swift helper"),
        );
    }

    if command_exists("podman") {
        return Some(
            Command::new("podman")
                .args([
                    "run",
                    "--rm",
                    "-i",
                    "-v",
                    &format!("{root}:/workspace"),
                    "-w",
                    "/workspace",
                    "swift:latest",
                    "swift",
                    "run",
                    "JetStreamInteropHelper",
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()
                .expect("failed to spawn podman swift helper"),
        );
    }

    eprintln!("SKIP: neither `swift` nor `podman` found — skipping Swift interop tests");
    None
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn swift_point_proptest(p in strategies::point_strategy()) {
        let mut child = match spawn_swift_helper() {
            Some(c) => c,
            None => return Ok(()),
        };
        let stdin = child.stdin.as_mut().unwrap();
        let stdout = child.stdout.as_mut().unwrap();

        roundtrip_one(stdin, stdout, TAG_POINT, &p).unwrap();

        write_end(stdin).unwrap();
        let status = child.wait().unwrap();
        prop_assert!(status.success());
    }

    #[test]
    fn swift_shape_proptest(s in strategies::shape_strategy()) {
        let mut child = match spawn_swift_helper() {
            Some(c) => c,
            None => return Ok(()),
        };
        let stdin = child.stdin.as_mut().unwrap();
        let stdout = child.stdout.as_mut().unwrap();

        roundtrip_one(stdin, stdout, TAG_SHAPE, &s).unwrap();

        write_end(stdin).unwrap();
        let status = child.wait().unwrap();
        prop_assert!(status.success());
    }

    #[test]
    fn swift_message_proptest(m in strategies::message_strategy()) {
        let mut child = match spawn_swift_helper() {
            Some(c) => c,
            None => return Ok(()),
        };
        let stdin = child.stdin.as_mut().unwrap();
        let stdout = child.stdout.as_mut().unwrap();

        roundtrip_one(stdin, stdout, TAG_MESSAGE, &m).unwrap();

        write_end(stdin).unwrap();
        let status = child.wait().unwrap();
        prop_assert!(status.success());
    }
}

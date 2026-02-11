// r[verify jetstream.interop.ts]
// r[verify jetstream.interop.roundtrip]
// r[verify jetstream.interop.byte-identical]
// r[verify jetstream.interop.proptest]
// r[verify jetstream.interop.protocol]

use jetstream_interop::*;
use proptest::prelude::*;
use std::path::Path;
use std::process::{Command, Stdio};

fn ts_helper_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/helpers/ts/interop-helper.ts")
}

fn tsx_bin() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/helpers/ts/node_modules/.bin/tsx")
}

/// Spawn the TS interop helper. Returns None if tsx is not installed.
fn spawn_ts_helper() -> Option<std::process::Child> {
    let tsx = tsx_bin();
    if !Path::new(&tsx).exists() {
        eprintln!("SKIP: tsx not found at {tsx} — run `pnpm install` first");
        return None;
    }
    Some(
        Command::new(&tsx)
            .arg(ts_helper_path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("failed to spawn tsx helper"),
    )
}

proptest! {
    #[test]
    fn ts_point_proptest(p in strategies::point_strategy()) {
        let mut child = match spawn_ts_helper() {
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
    fn ts_shape_proptest(s in strategies::shape_strategy()) {
        let mut child = match spawn_ts_helper() {
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
    fn ts_message_proptest(m in strategies::message_strategy()) {
        let mut child = match spawn_ts_helper() {
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

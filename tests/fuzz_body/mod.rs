//! What the `wireformat` fuzz target does to one input.
//!
//! Split out so the nightly fuzzer and the regression replay in
//! `tests/fuzz_regressions.rs` exercise the *same* code. A replay that
//! drifts from the target it replays is worse than none: it goes green
//! while the thing it claims to guard has changed underneath it.
//!
//! It lives under `tests/` rather than next to the fuzz target because
//! `fuzz/` is a nested package, which Cargo excludes wholesale when it
//! packages this crate. `tests/fuzz_regressions.rs` *is* packaged, so a
//! body kept in `fuzz/` left the published crate carrying a test that
//! includes a file the archive does not have — a compile error for
//! anyone running the published crate's suite. `tests/` is packaged, so
//! both sides of the include travel together.

use std::io::Cursor;

use jetstream::{p9::Tframe, prelude::*};

/// Decode an arbitrary byte string as a frame, and re-encode whatever
/// came back. Panicking here is the bug; failing to decode is not.
pub fn exercise(data: &[u8]) {
    if let Ok(tframe) = Tframe::decode(&mut Cursor::new(data)) {
        let _ = tframe.encode(&mut Vec::new());
    }

    struct TestMessage;
    impl Message for TestMessage {}
    impl WireFormat for TestMessage {
        fn byte_size(&self) -> u32 {
            0
        }

        fn encode<W: std::io::Write>(
            &self,
            _writer: &mut W,
        ) -> std::io::Result<()> {
            Ok(())
        }

        fn decode<R: std::io::Read>(_reader: &mut R) -> std::io::Result<Self> {
            Ok(TestMessage)
        }
    }

    if let Ok(test_message) = TestMessage::decode(&mut Cursor::new(data)) {
        let _ = test_message.encode(&mut Vec::new());
    }
}

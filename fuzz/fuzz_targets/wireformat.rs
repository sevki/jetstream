#![no_main]

use std::io::Cursor;

use jetstream::{p9::Tframe, prelude::*};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Fuzz test for jetstream_9p module
    if let Ok(tframe) = Tframe::decode(&mut Cursor::new(data)) {
        let _ = tframe.encode(&mut Vec::new());
    }

    // Fuzz test for jetstream_rpc module
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
});

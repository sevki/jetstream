//! A minimal protocol the session integration tests share.
//!
//! The session tests drive frames through lanes directly rather than
//! going through a generated service, because what is under test is the
//! session and lane machinery and not the code generator.

use std::io::{self, Read, Write};

use jetstream_rpc::{Error, Frame, Framer, Protocol};

/// A message whose body is whatever bytes are put in it, so that a test
/// can size a frame deliberately against a transport's datagram limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blob(pub Vec<u8>);

impl Blob {
    /// A blob of `len` bytes, filled with a repeating pattern.
    pub fn of(len: usize) -> Self {
        Blob((0..len).map(|n| n as u8).collect())
    }

    /// The blob as text, for a readable assertion.
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).expect("blob is not utf-8")
    }
}

impl Framer for Blob {
    fn message_type(&self) -> u8 {
        1
    }

    fn byte_size(&self) -> u32 {
        self.0.len() as u32
    }

    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.0)
    }

    fn decode<R: Read>(reader: &mut R, _ty: u8) -> io::Result<Self> {
        // `Frame::decode` hands down a reader bounded to this frame's
        // body, so reading it to the end reads exactly this message.
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        Ok(Blob(buf))
    }
}

#[derive(Debug)]
pub struct TestProtocol;

impl Protocol for TestProtocol {
    type Error = Error;
    type Request = Blob;
    type Response = Blob;

    const NAME: &'static str = "session-test";
    const VERSION: &'static str = "dev";
}

/// A frame carrying `body`.
pub fn frame(tag: u16, body: &str) -> Frame<Blob> {
    Frame {
        tag,
        msg: Blob(body.as_bytes().to_vec()),
    }
}

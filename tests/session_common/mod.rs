//! A minimal protocol the session integration tests share.
//!
//! The session tests drive frames through lanes directly rather than
//! going through a generated service, because what is under test is the
//! session and lane machinery and not the code generator.
//!
//! The request and response types are **deliberately distinct**, and
//! each refuses to decode the other's message type. A protocol that used
//! one type for both would let a session confuse the two directions and
//! still pass every test here — which is how the datagram API shipped
//! decoding a peer's requests as responses.

use std::io::{self, Read, Write};

use jetstream_rpc::{Error, Frame, Framer, Protocol};

const ASK: u8 = 1;
const SAY: u8 = 2;

/// What a caller sends. The body is whatever bytes are put in it, so
/// that a test can size a frame deliberately against a transport's
/// datagram limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ask(pub Vec<u8>);

/// What a callee sends back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Say(pub Vec<u8>);

/// `Frame::decode` hands down a reader bounded to this frame's body, so
/// reading it to the end reads exactly this message.
fn read_body<R: Read>(
    reader: &mut R,
    ty: u8,
    expected: u8,
    name: &str,
) -> io::Result<Vec<u8>> {
    if ty != expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("message type {ty} is not a {name}"),
        ));
    }
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    Ok(buf)
}

impl Ask {
    /// An ask of `len` bytes, filled with a repeating pattern.
    pub fn of(len: usize) -> Self {
        Ask((0..len).map(|n| n as u8).collect())
    }

    /// The body as text, for a readable assertion.
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).expect("body is not utf-8")
    }
}

impl Say {
    /// The body as text, for a readable assertion.
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).expect("body is not utf-8")
    }
}

impl Framer for Ask {
    fn message_type(&self) -> u8 {
        ASK
    }

    fn byte_size(&self) -> u32 {
        self.0.len() as u32
    }

    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.0)
    }

    fn decode<R: Read>(reader: &mut R, ty: u8) -> io::Result<Self> {
        read_body(reader, ty, ASK, "request").map(Ask)
    }
}

impl Framer for Say {
    fn message_type(&self) -> u8 {
        SAY
    }

    fn byte_size(&self) -> u32 {
        self.0.len() as u32
    }

    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.0)
    }

    fn decode<R: Read>(reader: &mut R, ty: u8) -> io::Result<Self> {
        read_body(reader, ty, SAY, "response").map(Say)
    }
}

#[derive(Debug)]
pub struct TestProtocol;

impl Protocol for TestProtocol {
    type Error = Error;
    type Request = Ask;
    type Response = Say;

    const NAME: &'static str = "session-test";
    const VERSION: &'static str = "dev";
}

/// A request frame carrying `body`.
pub fn frame(tag: u16, body: &str) -> Frame<Ask> {
    Frame {
        tag,
        msg: Ask(body.as_bytes().to_vec()),
    }
}

/// A response frame carrying `body`.
pub fn response(tag: u16, body: &str) -> Frame<Say> {
    Frame {
        tag,
        msg: Say(body.as_bytes().to_vec()),
    }
}

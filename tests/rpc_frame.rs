use std::io::Cursor;

use jetstream_rpc::{Frame, Framer};
use jetstream_wireformat::WireFormat;

#[derive(Clone)]
struct Dummy;

impl Framer for Dummy {
    fn message_type(&self) -> u8 { 0 }
    fn byte_size(&self) -> u32 { 0 }
    fn encode<W: std::io::Write>(&self, _: &mut W) -> std::io::Result<()> { Ok(()) }
    fn decode<R: std::io::Read>(_: &mut R, _: u8) -> std::io::Result<Self> { Ok(Dummy) }
}

#[test]
fn reject_too_small_byte_size() {
    let mut data = Vec::new();
    // byte_size smaller than header (should be 7 bytes)
    4u32.encode(&mut data).unwrap();
    // missing rest of header and body
    let res = Frame::<Dummy>::decode(&mut Cursor::new(data));
    assert!(res.is_err());
}

// r[impl jetstream.interop.protocol]
// r[impl jetstream.interop.roundtrip]
// r[impl jetstream.interop.byte-identical]

use jetstream_macros::JetStreamWireFormat;
use jetstream_wireformat::WireFormat;
use std::io::{self, Read, Write};

/// Type tags for the interop protocol.
pub const TAG_POINT: u8 = 1;
pub const TAG_SHAPE: u8 = 2;
pub const TAG_MESSAGE: u8 = 3;
pub const TAG_END: u8 = 0xFF;

/// A simple 2D point.
#[derive(Debug, Clone, PartialEq, JetStreamWireFormat)]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

/// A point with a color string.
#[derive(Debug, Clone, PartialEq, JetStreamWireFormat)]
pub struct ColorPoint {
    pub x: u32,
    pub y: u32,
    pub color: String,
}

/// An enum with named and unnamed variants.
#[derive(Debug, Clone, PartialEq, JetStreamWireFormat)]
pub enum Shape {
    Circle(u32),
    Rectangle { width: u32, height: u32 },
}

/// A message with a Vec and an Option.
#[derive(Debug, Clone, PartialEq, JetStreamWireFormat)]
pub struct Message {
    pub id: u32,
    pub tags: Vec<String>,
    pub payload: Option<String>,
}

/// r[jetstream.interop.proptest]
/// Proptest strategies for generating random test values.
pub mod strategies {
    use super::*;
    use proptest::prelude::*;

    pub fn point_strategy() -> impl Strategy<Value = Point> {
        (any::<u32>(), any::<u32>()).prop_map(|(x, y)| Point { x, y })
    }

    pub fn shape_strategy() -> impl Strategy<Value = Shape> {
        prop_oneof![
            any::<u32>().prop_map(Shape::Circle),
            (any::<u32>(), any::<u32>())
                .prop_map(|(width, height)| Shape::Rectangle { width, height }),
        ]
    }

    pub fn message_strategy() -> impl Strategy<Value = Message> {
        (
            any::<u32>(),
            proptest::collection::vec("[a-z]{1,8}", 0..4),
            proptest::option::of("[a-z]{1,16}"),
        )
            .prop_map(|(id, tags, payload)| Message {
                id,
                tags,
                payload,
            })
    }
}

/// Encode a value to bytes using WireFormat.
pub fn encode_value<T: WireFormat>(value: &T) -> Vec<u8> {
    let mut buf = Vec::with_capacity(value.byte_size() as usize);
    value
        .encode(&mut buf)
        .expect("encode to vec should not fail");
    buf
}

/// Write one interop frame: [type_tag: u8][length: u32 LE][payload].
pub fn write_frame<W: Write>(
    writer: &mut W,
    type_tag: u8,
    payload: &[u8],
) -> io::Result<()> {
    writer.write_all(&[type_tag])?;
    writer.write_all(&(payload.len() as u32).to_le_bytes())?;
    writer.write_all(payload)?;
    writer.flush()
}

/// Read one interop frame, returns (type_tag, payload).
/// Returns None if the type_tag is TAG_END.
pub fn read_frame<R: Read>(
    reader: &mut R,
) -> io::Result<Option<(u8, Vec<u8>)>> {
    let mut tag_buf = [0u8; 1];
    reader.read_exact(&mut tag_buf)?;
    let tag = tag_buf[0];
    if tag == TAG_END {
        return Ok(None);
    }
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload)?;
    Ok(Some((tag, payload)))
}

/// Write the end-of-test sentinel.
pub fn write_end<W: Write>(writer: &mut W) -> io::Result<()> {
    writer.write_all(&[TAG_END])?;
    writer.flush()
}

/// Round-trip a single value through the child process.
/// Sends encoded bytes, reads back the re-encoded bytes, asserts byte-identity.
pub fn roundtrip_one<T: WireFormat + std::fmt::Debug>(
    child_stdin: &mut impl Write,
    child_stdout: &mut impl Read,
    type_tag: u8,
    value: &T,
) -> io::Result<()> {
    let encoded = encode_value(value);
    write_frame(child_stdin, type_tag, &encoded)?;

    let frame = read_frame(child_stdout)?;
    match frame {
        Some((returned_tag, returned_bytes)) => {
            assert_eq!(
                returned_tag, type_tag,
                "type tag mismatch: expected {type_tag}, got {returned_tag}"
            );
            assert_eq!(
                encoded,
                returned_bytes,
                "byte mismatch for {value:?}: sent {} bytes, got {} bytes",
                encoded.len(),
                returned_bytes.len()
            );
            Ok(())
        }
        None => Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "child sent end-of-test before responding",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_encode_decode_roundtrip() {
        let p = Point { x: 42, y: 99 };
        let bytes = encode_value(&p);
        let decoded = Point::decode(&mut &bytes[..]).unwrap();
        assert_eq!(p, decoded);
    }

    #[test]
    fn shape_encode_decode_roundtrip() {
        let s1 = Shape::Circle(10);
        let bytes = encode_value(&s1);
        let decoded = Shape::decode(&mut &bytes[..]).unwrap();
        assert_eq!(s1, decoded);

        let s2 = Shape::Rectangle {
            width: 3,
            height: 7,
        };
        let bytes = encode_value(&s2);
        let decoded = Shape::decode(&mut &bytes[..]).unwrap();
        assert_eq!(s2, decoded);
    }

    #[test]
    fn message_encode_decode_roundtrip() {
        let m = Message {
            id: 1,
            tags: vec!["foo".into(), "bar".into()],
            payload: Some("hello".into()),
        };
        let bytes = encode_value(&m);
        let decoded = Message::decode(&mut &bytes[..]).unwrap();
        assert_eq!(m, decoded);
    }

    #[test]
    fn frame_write_read_roundtrip() {
        let mut buf = Vec::new();
        write_frame(&mut buf, TAG_POINT, &[1, 2, 3, 4]).unwrap();
        let frame = read_frame(&mut &buf[..]).unwrap();
        assert_eq!(frame, Some((TAG_POINT, vec![1, 2, 3, 4])));
    }

    #[test]
    fn end_sentinel() {
        let mut buf = Vec::new();
        write_end(&mut buf).unwrap();
        let frame = read_frame(&mut &buf[..]).unwrap();
        assert_eq!(frame, None);
    }
}

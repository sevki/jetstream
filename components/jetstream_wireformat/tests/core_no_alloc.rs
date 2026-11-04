//! Exercises the allocation free core against plain slices. Runs with and
//! without the `std` feature (`cargo test -p jetstream_wireformat
//! --no-default-features`); nothing here needs `alloc`.
use jetstream_wireformat::{JetStreamWireFormat, WireFormat};

#[derive(Debug, PartialEq, JetStreamWireFormat)]
struct Header {
    tag: u16,
    size: u32,
    flag: bool,
    extra: Option<u8>,
}

#[derive(Debug, PartialEq, JetStreamWireFormat)]
enum Kind {
    A,
    B(u64),
}

fn roundtrip<T: WireFormat + PartialEq + core::fmt::Debug>(v: T) {
    let mut buf = [0u8; 64];
    let mut out: &mut [u8] = &mut buf;
    v.encode(&mut out).unwrap();
    let written = 64 - out.len();
    assert_eq!(written as u32, v.byte_size());
    let mut input: &[u8] = &buf[..written];
    assert_eq!(T::decode(&mut input).unwrap(), v);
    assert!(input.is_empty());
}

#[test]
fn primitives_and_derives() {
    roundtrip((1u8, 0x1234u16, 0xdead_beefu32, -7i64, 1.5f32, true));
    roundtrip(Some(5u32));
    roundtrip(Header { tag: 3, size: 9, flag: true, extra: Some(1) });
    roundtrip(Kind::A);
    roundtrip(Kind::B(u64::MAX));
}

#[test]
fn wire_layout_is_little_endian() {
    let mut buf = [0u8; 4];
    let mut out: &mut [u8] = &mut buf;
    0x0102_0304u32.encode(&mut out).unwrap();
    assert_eq!(buf, [4, 3, 2, 1]);
}

#[test]
fn errors_instead_of_panics() {
    let mut short: &[u8] = &[1, 2];
    assert!(u32::decode(&mut short).is_err());
    let mut tiny = [0u8; 1];
    let mut out: &mut [u8] = &mut tiny;
    assert!(7u32.encode(&mut out).is_err());
    let mut bad: &[u8] = &[2];
    assert!(bool::decode(&mut bad).is_err());
    let mut bad: &[u8] = &[9, 0];
    assert!(Kind::decode(&mut bad).is_err());
}

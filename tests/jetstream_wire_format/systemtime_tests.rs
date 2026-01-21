//! Tests for SystemTime WireFormat implementation

use std::io::Cursor;
use std::time::{Duration, SystemTime};

use jetstream_wireformat::WireFormat;

fn round_trip<T: WireFormat + std::fmt::Debug + PartialEq>(value: T) -> T {
    let size = value.byte_size();
    let mut buffer = Vec::with_capacity(size as usize);
    value.encode(&mut buffer).expect("Failed to encode");

    assert_eq!(
        buffer.len(),
        size as usize,
        "Byte size calculation was incorrect"
    );

    let mut cursor = Cursor::new(buffer);
    let decoded: T = WireFormat::decode(&mut cursor).expect("Failed to decode");

    assert_eq!(cursor.position(), size as u64, "Didn't read all bytes");

    decoded
}

#[test]
fn test_systemtime_unix_epoch() {
    let original = SystemTime::UNIX_EPOCH;
    let decoded = round_trip(original);
    assert_eq!(decoded, original);
}

#[test]
fn test_systemtime_now() {
    // Note: We lose sub-millisecond precision in the encoding
    let now = SystemTime::now();
    let decoded = round_trip(now);

    // Check that the difference is less than 1ms (due to precision loss)
    let diff = now
        .duration_since(decoded)
        .or_else(|_| decoded.duration_since(now))
        .unwrap();
    assert!(diff < Duration::from_millis(1));
}

#[test]
fn test_systemtime_specific_time() {
    // 2024-01-01 00:00:00 UTC = 1704067200000 ms since epoch
    let time = SystemTime::UNIX_EPOCH + Duration::from_millis(1704067200000);
    let decoded = round_trip(time);
    assert_eq!(decoded, time);
}

#[test]
fn test_systemtime_byte_size() {
    let time = SystemTime::now();
    assert_eq!(time.byte_size(), 8); // u64
}

#[test]
fn test_systemtime_one_millisecond() {
    let time = SystemTime::UNIX_EPOCH + Duration::from_millis(1);
    let decoded = round_trip(time);
    assert_eq!(decoded, time);
}

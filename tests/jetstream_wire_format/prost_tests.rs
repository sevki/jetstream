//! Tests for the prost_wireformat macro

use std::io::Cursor;

use jetstream::prost_wireformat;
use jetstream_wireformat::WireFormat;
use prost::Message;

// Define a simple prost message for testing
#[derive(Clone, PartialEq, Message)]
pub struct TestMessage {
    #[prost(uint64, tag = "1")]
    pub id: u64,
    #[prost(string, tag = "2")]
    pub name: String,
    #[prost(bytes = "vec", tag = "3")]
    pub data: Vec<u8>,
}

// Public struct with private inner field and accessors
prost_wireformat!(pub TestMessage as TestMessageWrapper, derive(Debug));

// Public struct with public inner field and accessors
prost_wireformat!(pub TestMessage as pub PublicTestMessageWrapper, derive(Debug));

// Helper function for round-trip testing
fn round_trip<T: WireFormat + std::fmt::Debug>(value: T) -> T {
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
fn test_prost_wireformat_encode_decode() {
    let msg = TestMessage {
        id: 12345,
        name: "Hello, prost!".to_string(),
        data: vec![1, 2, 3, 4, 5],
    };

    let wrapper = TestMessageWrapper::new(msg.clone());
    let decoded = round_trip(wrapper);

    assert_eq!(decoded.inner().id, 12345);
    assert_eq!(decoded.inner().name, "Hello, prost!");
    assert_eq!(decoded.inner().data, vec![1, 2, 3, 4, 5]);
}

#[test]
fn test_prost_wireformat_accessors() {
    let msg = TestMessage {
        id: 42,
        name: "test".to_string(),
        data: vec![],
    };

    // Test new() and inner()
    let wrapper = TestMessageWrapper::new(msg);
    assert_eq!(wrapper.inner().id, 42);
    assert_eq!(wrapper.inner().name, "test");
}

#[test]
fn test_prost_wireformat_into_inner() {
    let msg = TestMessage {
        id: 100,
        name: "inner test".to_string(),
        data: vec![10, 20, 30],
    };

    let wrapper = TestMessageWrapper::new(msg);
    let inner = wrapper.into_inner();

    assert_eq!(inner.id, 100);
    assert_eq!(inner.name, "inner test");
    assert_eq!(inner.data, vec![10, 20, 30]);
}

#[test]
fn test_prost_wireformat_empty_message() {
    let msg = TestMessage {
        id: 0,
        name: String::new(),
        data: vec![],
    };

    let wrapper = TestMessageWrapper::new(msg);
    let decoded = round_trip(wrapper);

    assert_eq!(decoded.inner().id, 0);
    assert_eq!(decoded.inner().name, "");
    assert!(decoded.inner().data.is_empty());
}

#[test]
fn test_prost_wireformat_large_data() {
    let large_data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();

    let msg = TestMessage {
        id: u64::MAX,
        name: "a".repeat(1000),
        data: large_data.clone(),
    };

    let wrapper = TestMessageWrapper::new(msg);
    let decoded = round_trip(wrapper);

    assert_eq!(decoded.inner().id, u64::MAX);
    assert_eq!(decoded.inner().name.len(), 1000);
    assert_eq!(decoded.inner().data, large_data);
}

#[test]
fn test_prost_wireformat_pub_visibility() {
    // This test verifies the pub visibility variant compiles and works
    let msg = TestMessage {
        id: 1,
        name: "pub test".to_string(),
        data: vec![],
    };

    let wrapper = PublicTestMessageWrapper::new(msg);
    let decoded = round_trip(wrapper);

    assert_eq!(decoded.inner().id, 1);
}

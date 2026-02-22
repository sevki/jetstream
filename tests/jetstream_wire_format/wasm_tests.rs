//! WebAssembly-specific tests for the JetStreamWireFormat

#![cfg(target_arch = "wasm32")]

use jetstream_macros::JetStreamWireFormat;
use jetstream_wireformat::WireFormat;
use std::io::{Cursor, Read, Write};
use wasm_bindgen_test::wasm_bindgen_test;

#[derive(Debug, PartialEq, JetStreamWireFormat)]
struct WasmItem {
    id: u32,
    name: String,
    values: Vec<u64>,
}

// WebAssembly test implementation
#[wasm_bindgen_test]
fn test_wasm_encode_decode() {
    let item = WasmItem {
        id: 12345,
        name: "WebAssembly Test".to_string(),
        values: vec![1, 2, 3, 4, 5],
    };

    // Encode
    let mut buf = Vec::new();
    item.encode(&mut buf).expect("Failed to encode");

    // Decode
    let mut reader = Cursor::new(buf);
    let decoded: WasmItem =
        WireFormat::decode(&mut reader).expect("Failed to decode");

    // Verify
    assert_eq!(decoded.id, 12345);
    assert_eq!(decoded.name, "WebAssembly Test");
    assert_eq!(decoded.values, vec![1, 2, 3, 4, 5]);
    assert_eq!(decoded, item);
}

// Test generic support in WebAssembly
#[derive(Debug, PartialEq, JetStreamWireFormat)]
struct WasmGeneric<T: Clone> {
    value: T,
    count: u32,
}

#[wasm_bindgen_test]
fn test_wasm_generic() {
    let item = WasmGeneric {
        value: "Generic WebAssembly Test".to_string(),
        count: 42,
    };

    // Encode
    let mut buf = Vec::new();
    item.encode(&mut buf).expect("Failed to encode");

    // Decode
    let mut reader = Cursor::new(buf);
    let decoded: WasmGeneric<String> =
        WireFormat::decode(&mut reader).expect("Failed to decode");

    // Verify
    assert_eq!(decoded.value, "Generic WebAssembly Test");
    assert_eq!(decoded.count, 42);
    assert_eq!(decoded, item);
}

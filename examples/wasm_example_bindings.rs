//! WebAssembly example with explicit JavaScript bindings
//!
//! This example demonstrates how to use JetStream RPC with JavaScript bindings.

use jetstream_macros::JetStreamWireFormat;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// Define our message type
#[derive(Debug, JetStreamWireFormat)]
struct Message {
    id: u32,
    content: String,
}

// WebAssembly bindings
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn encode_message(id: u32, content: String) -> Vec<u8> {
    let msg = Message { id, content };
    let mut buffer = Vec::new();
    msg.encode(&mut buffer).expect("Failed to encode message");
    buffer
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn decode_message(buffer: &[u8]) -> String {
    let mut cursor = Cursor::new(buffer);
    match WireFormat::decode::<_, Message>(&mut cursor) {
        Ok(msg) => {
            format!("{{\"id\":{},\"content\":\"{}\"}}", msg.id, msg.content)
        }
        Err(_) => "{{\"error\":\"Failed to decode message\"}}".to_string(),
    }
}

// For native compilation
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("This example is intended for WebAssembly compilation.");
    println!("Use the following command to build for WebAssembly:");
    println!("cargo build --target wasm32-unknown-unknown --example wasm_example_bindings --release");
}

// Required for WebAssembly compilation
#[cfg(target_arch = "wasm32")]
fn main() {}

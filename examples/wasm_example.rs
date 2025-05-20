//! Example of WebAssembly compatibility for JetStream RPC
//! 
//! This example demonstrates how to use JetStream RPC in a WebAssembly context.

use jetstream_macros::JetStreamWireFormat;
use jetstream_wireformat::WireFormat;
use std::io::Cursor;

#[derive(Debug, JetStreamWireFormat)]
struct Message {
    id: u32,
    content: String,
}

// This function would be exported to JavaScript in a real WebAssembly binary
fn encode_message(id: u32, content: String) -> Vec<u8> {
    let msg = Message { id, content };
    let mut buffer = Vec::new();
    msg.encode(&mut buffer).expect("Failed to encode message");
    buffer
}

// This function would be exported to JavaScript in a real WebAssembly binary
fn decode_message(buffer: &[u8]) -> (u32, String) {
    let mut cursor = Cursor::new(buffer);
    let msg: Message = WireFormat::decode(&mut cursor).expect("Failed to decode message");
    (msg.id, msg.content)
}

// Simple example function
fn main() {
    // Create a message
    let encoded = encode_message(42, "Hello from WebAssembly!".to_string());
    println!("Encoded message: {:?}", encoded);
    
    // Decode the message
    let (id, content) = decode_message(&encoded);
    println!("Decoded message: id={}, content=\"{}\"", id, content);
}
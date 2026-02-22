//! WebAssembly specific implementation for jetstream_wireformat
//!
//! This module provides WebAssembly-specific functionality for the wire format.

#![cfg(target_arch = "wasm32")]

use crate::WireFormat;
use js_sys::{ArrayBuffer, Uint8Array};
use std::io::{self, Cursor};
use wasm_bindgen::prelude::*;

/// A trait that extends `WireFormat` with WebAssembly-specific methods.
pub trait WasmWireFormat: WireFormat {
    /// Convert this object to a JavaScript-compatible ArrayBuffer
    fn to_array_buffer(&self) -> Result<ArrayBuffer, JsValue> {
        let mut buffer = Vec::new();
        self.encode(&mut buffer)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let array = Uint8Array::new_with_length(buffer.len() as u32);
        array.copy_from(&buffer);
        Ok(array.buffer())
    }

    /// Convert from a JavaScript ArrayBuffer to this type
    fn from_array_buffer(buffer: &ArrayBuffer) -> Result<Self, JsValue> {
        let array = Uint8Array::new(buffer);
        let mut vec = vec![0; array.length() as usize];
        array.copy_to(&mut vec);
        let mut cursor = Cursor::new(vec);
        Self::decode(&mut cursor).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

// Implement WasmWireFormat for all types that implement WireFormat
impl<T: WireFormat> WasmWireFormat for T {}

/// A WebAssembly-compatible error wrapper
#[wasm_bindgen]
pub struct WasmError {
    message: String,
}

#[wasm_bindgen]
impl WasmError {
    #[wasm_bindgen(constructor)]
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.message.clone()
    }
}

impl From<io::Error> for WasmError {
    fn from(error: io::Error) -> Self {
        Self {
            message: error.to_string(),
        }
    }
}

/// Console logging helper for WebAssembly debugging
pub fn log(s: &str) {
    web_sys::console::log_1(&JsValue::from_str(s));
}

/// Error logging helper for WebAssembly debugging
pub fn error(s: &str) {
    web_sys::console::error_1(&JsValue::from_str(s));
}

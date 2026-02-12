mod codegen;
mod derive;

pub(crate) use derive::wire_format_inner;

// Re-export for tests
#[cfg(test)]
pub(crate) use codegen::{byte_size_sum, decode_wire_format, encode_wire_format};
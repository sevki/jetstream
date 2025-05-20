//! This test demonstrates both generic type support and field options in JetStreamWireFormat.

use std::io::{Cursor, Read, Write};
use std::marker::PhantomData;

use jetstream_wireformat::{JetStreamWireFormat, WireFormat};

// A custom codec used for demonstration
struct CustomCodec;

impl CustomCodec {
    // Byte size calculation
    fn byte_size(data: &Vec<u8>) -> u32 {
        // For demo purposes, just use the regular serialization
        (data.len() as u32) + 4 // 4 bytes for length prefix
    }
    
    // Encode data
    fn encode<W: Write>(data: &Vec<u8>, writer: &mut W) -> std::io::Result<()> {
        // Write length + data
        WireFormat::encode(&(data.len() as u32), writer)?;
        writer.write_all(data)
    }
    
    // Decode data
    fn decode<R: Read>(reader: &mut R) -> std::io::Result<Vec<u8>> {
        // Read length then data
        let len: u32 = WireFormat::decode(reader)?;
        let mut buf = vec![0u8; len as usize];
        reader.read_exact(&mut buf)?;
        Ok(buf)
    }
}

// Helper functions for type conversion
fn into_wire_format(value: &String) -> Vec<u8> {
    value.as_bytes().to_vec()
}

fn from_wire_format(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).unwrap_or_default()
}


// A wrapper type to test generic support
#[derive(Debug, PartialEq, Clone, JetStreamWireFormat)]
struct Wrapper<T: Clone> {
    value: T,
}

// A complex struct with both generics and field options
#[derive(Debug, PartialEq, Clone, JetStreamWireFormat)]
struct ComplexMessage<T: Clone, U: Clone> {
    id: u32,
    #[jetstream(with(CustomCodec))]
    custom_data: Vec<u8>,
    #[jetstream(into(self::into_wire_format))]
    converted_field: String,
    #[jetstream(from(self::from_wire_format))]
    converted_back: String,
    #[jetstream(into(self::into_wire_format))]
    as_converted: String,
    generic_field_t: T,
    generic_field_u: U,
    #[jetstream(skip)]
    skipped_field: u64,
}

// An enum with generics and field options
#[derive(Debug, PartialEq, Clone, JetStreamWireFormat)]
enum GenericEnum<T: Clone, U: Clone> {
    Plain(u32),
    WithCustom {
        #[jetstream(with(self::CustomCodec))]
        data: Vec<u8>,
    },
    WithGeneric(T),
    WithBoth(U, #[jetstream(with(self::CustomCodec))] Vec<u8>),
}

// Helper function for round-trip testing
fn round_trip<T: WireFormat + PartialEq + std::fmt::Debug>(value: T) -> T {
    // Serialize
    let size = value.byte_size();
    let mut buffer = Vec::with_capacity(size as usize);
    value.encode(&mut buffer).expect("Failed to encode");
    
    // Verify the size calculation was correct
    assert_eq!(buffer.len(), size as usize, "Byte size calculation was incorrect");
    
    // Deserialize
    let mut cursor = Cursor::new(buffer);
    let decoded: T = WireFormat::decode(&mut cursor).expect("Failed to decode");
    
    // Verify cursor position reached the end
    assert_eq!(cursor.position(), size as u64, "Didn't read all bytes");
    
    decoded
}

#[test]
fn test_generic_wrapper() {
    // Test with a simple u32
    let original = Wrapper { value: 42u32 };
    let decoded = round_trip(original.clone());
    assert_eq!(decoded, original);
    
    // Test with a string
    let original = Wrapper { value: "Hello, world!".to_string() };
    let decoded = round_trip(original.clone());
    assert_eq!(decoded, original);
    
    // Test with a nested wrapper
    let original = Wrapper { 
        value: Wrapper { value: 42u32 } 
    };
    let decoded = round_trip(original.clone());
    assert_eq!(decoded, original);
}

#[test]
fn test_complex_message() {
    let original = ComplexMessage {
        id: 12345,
        custom_data: vec![1, 2, 3, 4, 5],
        converted_field: "Hello".to_string(),
        converted_back: "World".to_string(),
        as_converted: "Test".to_string(),
        generic_field_t: 42u32,
        generic_field_u: "Generic U".to_string(),
        skipped_field: 0xDEADBEEF, // This should be set to default (0) in the decoded version
    };
    
    let decoded = round_trip(original.clone());
    
    // The skipped field should be set to default
    let mut expected = original.clone();
    expected.skipped_field = 0;
    
    assert_eq!(decoded, expected);
}

#[test]
fn test_generic_enum() {
    // Test Plain variant
    let original: GenericEnum<u32, u32> = GenericEnum::Plain(42);
    let decoded = round_trip(original.clone());
    assert_eq!(decoded, original);
    
    // Test WithCustom variant
    let original: GenericEnum<u32, u32> = GenericEnum::WithCustom { 
        data: vec![1, 2, 3, 4, 5] 
    };
    let decoded = round_trip(original.clone());
    assert_eq!(decoded, original);
    
    // Test WithGeneric variant with u32
    let original: GenericEnum<u32, String> = GenericEnum::WithGeneric(42);
    let decoded = round_trip(original.clone());
    assert_eq!(decoded, original);
    
    // Test WithGeneric variant with String
    let original: GenericEnum<String, u32> = GenericEnum::WithGeneric("Hello".to_string());
    let decoded = round_trip(original.clone());
    assert_eq!(decoded, original);
    
    // Test WithBoth variant
    let original: GenericEnum<u32, String> = GenericEnum::WithBoth(
        "Hello".to_string(), 
        vec![1, 2, 3, 4, 5]
    );
    let decoded = round_trip(original.clone());
    assert_eq!(decoded, original);
}

// Test nested generic types
#[derive(Debug, PartialEq, Clone, JetStreamWireFormat)]
struct NestedGeneric<T: Clone, U: Clone> {
    outer: Wrapper<T>,
    inner: Wrapper<U>,
}

#[test]
fn test_nested_generics() {
    let original = NestedGeneric {
        outer: Wrapper { value: 42u32 },
        inner: Wrapper { value: "Hello".to_string() },
    };
    
    let decoded = round_trip(original.clone());
    assert_eq!(decoded, original);
}

// Test with phantom data to ensure generics work even with unused type parameters
// PhantomData doesn't implement WireFormat so we need to handle it specially
#[derive(Debug, PartialEq, Clone)]
struct WithPhantom<T: Clone, U> {
    value: T,
    _phantom: PhantomData<U>,
}

// Manually implement WireFormat for WithPhantom since PhantomData doesn't implement it
#[cfg(not(target_arch = "wasm32"))]
impl<T: Clone + WireFormat, U: Send> WireFormat for WithPhantom<T, U> {
    fn byte_size(&self) -> u32 {
        self.value.byte_size()
    }

    fn encode<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.value.encode(writer)
    }

    fn decode<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let value = T::decode(reader)?;
        Ok(WithPhantom {
            value,
            _phantom: PhantomData,
        })
    }
}

// WebAssembly version that doesn't require Send
#[cfg(target_arch = "wasm32")]
impl<T: Clone + WireFormat, U> WireFormat for WithPhantom<T, U> {
    fn byte_size(&self) -> u32 {
        self.value.byte_size()
    }

    fn encode<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.value.encode(writer)
    }

    fn decode<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let value = T::decode(reader)?;
        Ok(WithPhantom {
            value,
            _phantom: PhantomData,
        })
    }
}

#[test]
fn test_phantom_data() {
    let original: WithPhantom<u32, String> = WithPhantom {
        value: 42,
        _phantom: PhantomData,
    };
    
    let decoded = round_trip(original.clone());
    assert_eq!(decoded, original);
}
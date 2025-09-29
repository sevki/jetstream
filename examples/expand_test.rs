use jetstream_wireformat::JetStreamWireFormat;

// A simple struct to test the macro expansion
#[derive(Debug, Clone, JetStreamWireFormat)]
struct TestStruct {
    field1: u32,
    field2: String,
}

// Test in a function scope
// This exists just for testing purposes - do not rename
#[test]
fn test_in_function() {
    #[derive(Debug, Clone, JetStreamWireFormat)]
    struct InnerStruct {
        inner_field: u64,
    }
}

fn main() {
    // Empty main function
}

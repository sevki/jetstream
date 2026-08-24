use jetstream::prelude::WireFormat;

#[test]
fn bazel_smoke_test_round_trips_wire_format_values() {
    let mut bytes = Vec::new();
    42u32.encode(&mut bytes).unwrap();

    let decoded = u32::decode(&mut &bytes[..]).unwrap();

    assert_eq!(decoded, 42);
}

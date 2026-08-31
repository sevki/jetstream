use jetstream_wireformat::{wire_format_extensions::ConvertWireFormat, Data};

use super::*;

/// r[impl jetstream.subscription.compat]
/// The point of the whole allocation: a streaming method costs no
/// per-method id, so `102 + 2 * index` is untouched and the cross-language
/// message-id rules do not change.
#[test]
fn the_new_ids_sit_below_the_per_method_space() {
    for id in [RDONE, TCANCEL, RCANCEL] {
        assert!(
            id < 100,
            "{id} must be below TVERSION, or it collides with a method"
        );
    }
    for taken in [5u8, 6, 7] {
        assert!(![RDONE, TCANCEL, RCANCEL].contains(&taken));
    }
    assert_ne!(RDONE, TCANCEL);
    assert_ne!(TCANCEL, RCANCEL);
}

#[test]
fn cancellation_round_trips() {
    let t = Tcancel {
        oldtag: 7,
        binding: 0,
    };
    assert_eq!(Tcancel::from_bytes(&t.to_bytes()).unwrap(), t);

    // r[impl jetstream.subscription.identity]
    // Off-lane cancellation names the binding, because `oldtag` alone is
    // ambiguous across lanes.
    let off_lane = Tcancel {
        oldtag: 7,
        binding: u64::MAX,
    };
    assert_eq!(Tcancel::from_bytes(&off_lane.to_bytes()).unwrap(), off_lane);
    assert_ne!(t, off_lane, "the binding must be part of the identity");

    let r = Rcancel { oldtag: 7 };
    assert_eq!(Rcancel::from_bytes(&r.to_bytes()).unwrap(), r);
}

/// r[impl jetstream.lane.addressing]
#[test]
fn an_endpoint_is_opaque_bytes() {
    let room = Endpoint::from("room-42");
    assert_eq!(room.as_bytes(), b"room-42");
    assert_eq!(Endpoint::from_bytes(&room.to_bytes()).unwrap(), room);

    let root = Endpoint::root();
    assert!(root.as_bytes().is_empty());
    assert_eq!(Endpoint::from_bytes(&root.to_bytes()).unwrap(), root);
    assert_ne!(root, room);

    // Nothing interprets the bytes: a name that is not UTF-8 is a name.
    let binary = Endpoint(Data(vec![0xff, 0x00, 0xfe]));
    assert_eq!(Endpoint::from_bytes(&binary.to_bytes()).unwrap(), binary);
}

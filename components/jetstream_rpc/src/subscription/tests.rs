use jetstream_wireformat::{wire_format_extensions::ConvertWireFormat, Data};

use super::*;

/// r[impl jetstream.subscription.identity]
/// Zero is reserved, so an off-lane cancellation can never be mistaken
/// for an on-lane one. The specification used to recommend a counter
/// starting at zero, which made the first subscription of every session
/// exactly that mistake.
#[test]
fn zero_is_not_a_binding() {
    let on_lane = Tcancel::on_lane(7);
    assert_eq!(on_lane.binding, 0);
    assert_eq!(on_lane.target_binding(), None, "no binding is named");

    let off_lane = Tcancel::off_lane(7, 1);
    assert_eq!(off_lane.target_binding(), Some(1));
    assert_ne!(
        on_lane, off_lane,
        "the first binding of a session must not encode as the on-lane form"
    );
}

/// The allocation the specification used to bless, refused at the
/// constructor rather than producing a cancellation nobody can route.
#[test]
#[should_panic(expected = "reserved for the on-lane case")]
fn a_binding_counter_starting_at_zero_is_refused() {
    let _ = Tcancel::off_lane(7, 0);
}

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

// ---------------------------------------------------------------------------
// The typed surface.

use futures::StreamExt;

use super::{channel, merge, Item, Subscription, Terminator};

/// r[impl jetstream.subscription.surface.terminal-value]
/// The end carries a value. A `Stream` alone ends with `None`, which is
/// where the first attempt at this surface stopped: `Subscription<Event>`
/// offered no `result()` and could not be given one.
#[tokio::test]
async fn the_end_carries_a_value() {
    let cancel = tokio_util::sync::CancellationToken::new();
    let (producer, items) = channel::<u32, String>(8, cancel);
    tokio::spawn(async move {
        producer.send(1).await.unwrap();
        producer.send(2).await.unwrap();
        producer.finish("two messages".to_string()).await;
    });

    let got: Vec<Item<u32, String>> = items.collect().await;
    assert_eq!(
        got,
        vec![
            Item::Next(1),
            Item::Next(2),
            Item::Done("two messages".to_string())
        ]
    );
}

/// r[impl jetstream.subscription.surface.producer]
/// A producer that watches nothing still stops, because the send is the
/// third of the three forms the rule allows. The loop below is the one
/// that used to run forever.
#[tokio::test]
async fn a_producer_that_watches_nothing_still_stops() {
    let cancel = tokio_util::sync::CancellationToken::new();
    let (producer, mut items) = channel::<u32, ()>(1, cancel.clone());
    let looping = tokio::spawn(async move {
        let mut sent = 0u32;
        // No `is_cancelled`, no `cancelled().await`. Just work and send.
        while producer.send(sent).await.is_ok() {
            sent += 1;
        }
        sent
    });

    assert_eq!(items.next().await, Some(Item::Next(0)));
    cancel.cancel();
    drop(items);

    let sent = tokio::time::timeout(std::time::Duration::from_secs(5), looping)
        .await
        .expect("a cancelled producer must stop at its next send")
        .unwrap();
    assert!(sent < 1_000, "it stopped rather than ran on: {sent}");
}

/// r[impl jetstream.subscription.surface.producer]
/// And a producer that *does* watch can stop between items, which is
/// what a subscription whose work is expensive needs.
#[tokio::test]
async fn a_producer_can_watch_for_cancellation() {
    let cancel = tokio_util::sync::CancellationToken::new();
    let (producer, _items) = channel::<u32, ()>(8, cancel.clone());
    assert!(!producer.is_cancelled());
    cancel.cancel();
    assert!(producer.is_cancelled());
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        producer.cancelled(),
    )
    .await
    .expect("the awaited signal must fire too");
}

fn finite(items: Vec<u32>, done: &str) -> Subscription<u32, String> {
    let done = done.to_string();
    Subscription::from_items(futures::stream::iter(
        items
            .into_iter()
            .map(Item::Next)
            .chain(std::iter::once(Item::Done(done)))
            .map(Ok),
    ))
}

/// r[impl jetstream.subscription.surface.composition]
/// Fan-in without losing the ends. `select_all` over bare item streams
/// yields every item and no terminator, and cannot say which sequence
/// finished — use case 2 and use case 4 are each satisfiable alone and
/// were not satisfiable together.
#[tokio::test]
async fn merging_keeps_every_terminator_and_says_whose() {
    let merged = merge([
        ("east", finite(vec![1, 2], "east closed")),
        ("west", finite(vec![3], "west closed")),
    ]);

    let mut items: Vec<(&str, u32)> = Vec::new();
    let mut ends: Vec<(&str, String)> = Vec::new();
    let mut merged = merged;
    while let Some(next) = merged.next().await {
        match next.unwrap() {
            (who, Item::Next(n)) => items.push((who, n)),
            (who, Item::Done(why)) => ends.push((who, why)),
        }
    }

    items.sort();
    ends.sort();
    assert_eq!(items, vec![("east", 1), ("east", 2), ("west", 3)]);
    assert_eq!(
        ends,
        vec![
            ("east", "east closed".to_string()),
            ("west", "west closed".to_string())
        ],
        "both ends must survive the merge, and be attributable"
    );
}

/// r[impl jetstream.subscription.termination]
/// Two subscriptions on one protocol have two terminal types under one
/// global `RDONE`. The payload names its method, so a decoder that never
/// sees the tag can still tell them apart.
#[test]
fn a_typed_terminator_names_its_method() {
    let closed = Terminator {
        method: 102,
        value: 7u32,
    };
    let other = Terminator {
        method: 104,
        value: 7u32,
    };
    assert_ne!(encoded(&closed), encoded(&other));

    let mut bytes = encoded(&closed);
    let round: Terminator<u32> =
        WireFormat::decode(&mut bytes.as_slice()).unwrap();
    assert_eq!(round, closed);
    assert_eq!(
        closed.byte_size() as usize,
        bytes.len(),
        "the discriminant costs exactly one byte"
    );
    bytes.clear();
}

fn encoded<T: WireFormat>(v: &T) -> Vec<u8> {
    let mut out = Vec::new();
    v.encode(&mut out).unwrap();
    out
}

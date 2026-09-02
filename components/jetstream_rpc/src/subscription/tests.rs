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

    let got: Vec<Item<u32, String>> = items
        .map(|item| item.expect("this producer does not fail"))
        .collect()
        .await;
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

    assert_eq!(items.next().await.map(|i| i.unwrap()), Some(Item::Next(0)));
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
        match next {
            (who, Ok(Item::Next(n))) => items.push((who, n)),
            (who, Ok(Item::Done(why))) => ends.push((who, why)),
            (who, Err(e)) => panic!("{who} failed unexpectedly: {e}"),
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

/// r[impl jetstream.subscription.surface.composition]
/// A failure keeps its key too.
///
/// Merging discarded the key on `Err`, so in a fan-in over several rooms
/// a caller could see that *something* failed and not which — it could
/// neither retry that room nor report it. That is the same loss the rule
/// forbids for a successful end, and it went unnoticed because every
/// test merged subscriptions that only succeed.
#[tokio::test]
async fn merging_says_which_subscription_failed() {
    let failing: Subscription<u32, String> =
        Subscription::from_items(futures::stream::iter(vec![
            Ok(Item::Next(1)),
            Err(jetstream_error::Error::new("the room went away")),
        ]));
    let merged =
        merge([("east", finite(vec![9], "east closed")), ("west", failing)]);

    let mut failures: Vec<(&str, String)> = Vec::new();
    let mut merged = merged;
    while let Some((who, result)) = merged.next().await {
        if let Err(e) = result {
            failures.push((who, e.to_string()));
        }
    }

    assert_eq!(failures.len(), 1, "exactly one input failed");
    assert_eq!(failures[0].0, "west", "and the caller can tell which");
    assert!(failures[0].1.contains("the room went away"));
}

/// r[impl jetstream.subscription.surface.establishment]
/// Abandoning `establish()` must leave the subscription usable.
///
/// It moved the opening future out and left a placeholder behind, so
/// dropping this future — a timeout, a losing `select!` branch, which is
/// reachable whenever opening has to wait for a tag — dropped the only
/// opening future with it. The subscription was then permanently stuck:
/// a second `establish()` silently did nothing, and the next poll hit an
/// `unreachable!`.
///
/// Tested against an open that is *held* pending on purpose. In process
/// the open usually completes on its first poll, so a test that merely
/// raced a short timeout passed with the bug in place — which is how
/// this nearly went in unverified.
#[tokio::test]
async fn an_abandoned_establish_leaves_the_subscription_usable() {
    use futures::FutureExt;

    let (unblock, blocked) = tokio::sync::oneshot::channel::<()>();
    let mut subscription: Subscription<u32, ()> =
        Subscription::opening(async move {
            let _ = blocked.await;
            Subscription::from_items(futures::stream::iter(vec![Ok(
                Item::Next(7),
            )]))
        });

    // Abandoned while pending: the open cannot finish yet.
    assert!(
        subscription.establish().now_or_never().is_none(),
        "the open must still be pending for this to test anything"
    );

    // Now let it finish. The subscription must be exactly as it was.
    unblock.send(()).unwrap();
    subscription.establish().await;
    assert!(
        matches!(subscription.next().await, Some(Ok(Item::Next(7)))),
        "an abandoned establishment must not consume the subscription"
    );
}

/// r[impl jetstream.subscription.surface.termination]
/// A producer that fails must be able to say so. Before the channel
/// carried failures it could not: returning early — the `?` in any
/// ordinary producer loop — dropped the `Producer`, closing the channel,
/// which the dispatcher reads as a subscription that merely stopped. It
/// then supplies the terminator the caller is owed, and the subscriber
/// receives a normal typed ending carrying a *fabricated* result. A
/// failure reported as success is the one outcome the surface must never
/// produce.
#[tokio::test]
async fn a_failed_producer_does_not_report_a_normal_ending() {
    let cancel = tokio_util::sync::CancellationToken::new();
    let (producer, items) = channel::<u32, String>(8, cancel);
    tokio::spawn(async move {
        producer.send(1).await.unwrap();
        producer
            .fail(jetstream_error::Error::new("the feed went away"))
            .await;
    });

    let got: Vec<_> = items.collect().await;
    assert!(
        matches!(got.first(), Some(Ok(Item::Next(1)))),
        "the item before the failure still arrives: {got:?}",
    );
    let last = got.last().expect("the failure is an item in the sequence");
    assert!(
        last.is_err(),
        "the sequence must end in the failure, not in silence: {got:?}",
    );
    assert!(
        !got.iter().any(|i| matches!(i, Ok(Item::Done(_)))),
        "a failure must never be dressed up as a completed subscription: \
         {got:?}",
    );
}

/// r[impl jetstream.subscription.surface.cancellation]
/// A subscription that opened onto its own producer and is then *served*
/// must not have that producer cancelled out from under it.
///
/// `serve` used to match on `self.source` alone, leaving the rest of the
/// subscription — including the guard installed by `establish` or a poll
/// — to drop as it returned. The guard fired immediately, so the stream
/// handed to the dispatcher was already dead: a handler returning a
/// pre-established subscription would serve nothing at all.
#[tokio::test]
async fn serving_an_opened_subscription_keeps_its_producer() {
    let alive = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let theirs = alive.clone();

    let mut subscription = Subscription::<u32, ()>::opening(async move {
        Subscription::producing(4, move |producer| async move {
            theirs.store(true, std::sync::atomic::Ordering::SeqCst);
            // Stops only when cancelled, so the test can tell the
            // difference between "still producing" and "already gone".
            let mut n = 0u32;
            while producer.send(n).await.is_ok() {
                n += 1;
            }
            theirs.store(false, std::sync::atomic::Ordering::SeqCst);
        })
    });

    // Opening installs the guard on the subscription itself.
    subscription.establish().await;

    // The dispatcher's call. Its own token is separate; nothing has
    // cancelled it.
    let mut items = subscription.serve(CancellationToken::new());

    let first =
        tokio::time::timeout(std::time::Duration::from_secs(5), items.next())
            .await
            .expect("a served producer must still be producing");
    assert!(
        matches!(first, Some(Ok(Item::Next(0)))),
        "expected the producer's first item, got {first:?}",
    );
    assert!(
        alive.load(std::sync::atomic::Ordering::SeqCst),
        "serving must not cancel the producer it is serving",
    );
}

/// r[verify jetstream.subscription.cancel]
/// The dispatcher's token has to reach a sequence that is *already* a
/// sequence, because a service that proxies or delegates a subscription
/// serves one backed by real work.
///
/// This is the failing direction of the arm that used to ignore it: a
/// pre-established subscription over a live producer, cancelled through
/// the token rather than by dropping the stream. Before the fix the
/// stream simply never ended, which is how the tag leaked — the
/// dispatcher only frees a tag when it sees the end.
#[tokio::test]
async fn cancelling_ends_a_served_open_subscription() {
    let mut subscription = Subscription::<u32, ()>::opening(async move {
        Subscription::producing(4, move |producer| async move {
            let mut n = 0u32;
            while producer.send(n).await.is_ok() {
                n += 1;
            }
        })
    });
    // `establish` is what makes this `Source::Open` rather than
    // `Source::Opening` — the arm under test.
    subscription.establish().await;

    let cancel = CancellationToken::new();
    let mut items = subscription.serve(cancel.clone());

    assert!(
        matches!(items.next().await, Some(Ok(Item::Next(0)))),
        "the producer must be running before cancellation means anything",
    );

    cancel.cancel();

    // Not "eventually quiet": the sequence has to *end*, because the end
    // is the signal. `server::run` answers a caller and frees the tag on
    // `Out::Ended`, and on nothing else.
    let ended =
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            // Items already in flight may still arrive; the end must.
            while let Some(item) = items.next().await {
                if item.is_err() {
                    return false;
                }
            }
            true
        })
        .await
        .expect("a cancelled subscription must end, not hang");
    assert!(ended, "the sequence ended cleanly");
}

/// r[verify jetstream.subscription.cancel]
/// Cancelling reaches a subscription that is still opening, too — the
/// other arm that ignored the token.
#[tokio::test]
async fn cancelling_ends_a_served_opening_subscription() {
    let subscription = Subscription::<u32, ()>::opening(async move {
        Subscription::producing(4, move |producer| async move {
            let mut n = 0u32;
            while producer.send(n).await.is_ok() {
                n += 1;
            }
        })
    });

    let cancel = CancellationToken::new();
    // Served without `establish`, so the opening future is still the
    // source and the flattened stream is what has to observe the token.
    let mut items = subscription.serve(cancel.clone());

    assert!(
        matches!(items.next().await, Some(Ok(Item::Next(0)))),
        "the nested producer must be running",
    );

    cancel.cancel();

    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while items.next().await.is_some() {}
    })
    .await
    .expect("a cancelled subscription must end, not hang");
}

/// A subscription that finished on its own is not a subscription anyone
/// can cancel. Cancelling afterwards must be a no-op — not a second
/// ending, and not something that rewrites what the caller already saw.
///
/// This guards the truncation `until_cancelled` introduces: it must
/// apply to work that is still running, never to a sequence that has
/// already delivered its terminator.
#[tokio::test]
async fn cancelling_after_the_end_changes_nothing() {
    let subscription =
        Subscription::<u32, ()>::from_items(futures::stream::iter(vec![
            Ok(Item::Next(1)),
            Ok(Item::Next(2)),
            Ok(Item::Done(())),
        ]));

    let cancel = CancellationToken::new();
    let mut items = subscription.serve(cancel.clone());

    let mut seen = Vec::new();
    while let Some(Ok(item)) = items.next().await {
        seen.push(item);
    }
    assert_eq!(
        seen,
        vec![Item::Next(1), Item::Next(2), Item::Done(())],
        "the whole sequence, terminator included",
    );

    // After the fact. Nothing left to cancel, and nothing that should
    // happen because of it.
    cancel.cancel();
    assert!(
        items.next().await.is_none(),
        "an ended subscription stays ended",
    );
}

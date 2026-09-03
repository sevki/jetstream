//! A generated subscription, end to end over a session.
//!
//! The example proves the shape runs; these prove the individual
//! promises, one at a time, and fail loudly rather than hanging.

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering::SeqCst},
        Arc,
    },
    time::Duration,
};

use futures::{FutureExt, StreamExt};
use jetstream::prelude::*;
use jetstream_rpc::session::{LocalSession, Session};
use tokio::sync::broadcast;

use crate::counter_protocol::{CounterChannel, CounterService};

#[derive(Debug, Clone, PartialEq, Eq, JetStreamWireFormat)]
pub struct Tick(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, JetStreamWireFormat)]
pub struct Total(pub u64);

#[service(uses(super::{Tick, Total}))]
pub trait Counter {
    /// Unary, and it must keep working while a subscription is open.
    async fn bump(&self, ctx: Context) -> Result<u64>;

    /// Streaming: every bump from now on, until the counter stops.
    #[subscription]
    fn ticks(&self, ctx: Context) -> Subscription<Tick, Total>;
}

#[derive(Clone)]
struct Counting {
    ticks: broadcast::Sender<Tick>,
    stop: subscription::CancellationToken,
    /// How many producers are alive: the only way to see that
    /// cancellation reached the work rather than only the delivery.
    producers: Arc<AtomicUsize>,
    seen: Arc<AtomicUsize>,
}

impl Counter for Counting {
    async fn bump(&self, _ctx: Context) -> Result<u64> {
        let n = self.seen.fetch_add(1, SeqCst) as u64 + 1;
        let _ = self.ticks.send(Tick(n));
        Ok(n)
    }

    fn ticks(&self, _ctx: Context) -> Subscription<Tick, Total> {
        let mut feed = self.ticks.subscribe();
        let stop = self.stop.clone();
        let seen = self.seen.clone();
        let producers = self.producers.clone();
        Subscription::producing(16, move |producer| async move {
            producers.fetch_add(1, SeqCst);
            let ending = loop {
                tokio::select! {
                    biased;
                    _ = producer.cancelled() => break None,
                    got = feed.recv() => match got {
                        Ok(tick) => {
                            if producer.send(tick).await.is_err() {
                                break None;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(_) => break Some(Total(seen.load(SeqCst) as u64)),
                    },
                    _ = stop.cancelled() => {
                        break Some(Total(seen.load(SeqCst) as u64))
                    }
                }
            };
            if let Some(total) = ending {
                producer.finish(total).await;
            }
            producers.fetch_sub(1, SeqCst);
        })
    }
}

struct Wired {
    session: LocalSession<CounterChannel>,
    producers: Arc<AtomicUsize>,
    stop: subscription::CancellationToken,
}

impl Wired {
    fn new() -> Self {
        let pair = LocalSession::<CounterChannel>::pair();
        let counting = Counting {
            ticks: broadcast::channel(64).0,
            stop: subscription::CancellationToken::new(),
            producers: Default::default(),
            seen: Default::default(),
        };
        let producers = counting.producers.clone();
        let stop = counting.stop.clone();
        let server = pair.server.clone();
        tokio::spawn(async move {
            while let Ok(lane) =
                Session::<CounterChannel>::accept_lane(&server).await
            {
                let inner = counting.clone();
                tokio::spawn(async move {
                    let mut service = CounterService { inner };
                    let _ =
                        jetstream_rpc::server::run(&mut service, lane).await;
                });
            }
        });
        Wired {
            session: pair.client,
            producers,
            stop,
        }
    }

    async fn lane(&self) -> CounterChannel {
        let lane = Session::<CounterChannel>::open_lane(&self.session)
            .await
            .expect("the session is open");
        CounterChannel::new(16, Box::new(lane))
    }
}

async fn settle() {
    tokio::time::sleep(Duration::from_millis(50)).await;
}

/// Wait for a condition rather than sleeping and hoping.
async fn until(what: &str, mut cond: impl FnMut() -> bool) {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !cond() {
        assert!(std::time::Instant::now() < deadline, "timed out: {what}");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// r[verify jetstream.subscription.overview]
/// One request, many responses.
#[tokio::test]
async fn a_generated_subscription_delivers_many_items() {
    let wired = Wired::new();
    let mut ticks = wired.lane().await.ticks(Context::default());
    let bumper = wired.lane().await;

    // A subscription opens when it is first read, so without this the
    // bumps below would happen before the service had heard of it.
    ticks.establish().await;
    // Even established, this subscription is on its own lane and the
    // bumps are on another, so it is only *this* producer being live
    // that makes the ticks arrive — see `until` below.
    until("the producer must start", || {
        wired.producers.load(SeqCst) == 1
    })
    .await;

    for _ in 0..3 {
        bumper.bump(Context::default()).await.unwrap();
    }

    let mut got = Vec::new();
    while got.len() < 3 {
        let item = tokio::time::timeout(Duration::from_secs(5), ticks.next())
            .await
            .expect("three items must arrive")
            .expect("the subscription must not end");
        match item.unwrap() {
            Item::Next(Tick(n)) => got.push(n),
            Item::Done(_) => panic!("ended early"),
        }
    }
    assert_eq!(got, vec![1, 2, 3]);
}

/// r[verify jetstream.subscription.dispatch.concurrent]
/// A subscription is one call among the lane's others. Both go on the
/// *same* lane here, so a dispatcher that served the subscription by
/// consuming it would never answer the bump.
#[tokio::test]
async fn a_unary_call_works_on_a_lane_serving_a_subscription() {
    let wired = Wired::new();
    let channel = wired.lane().await;
    let mut ticks = channel.ticks(Context::default());
    ticks.establish().await;

    let n = tokio::time::timeout(
        Duration::from_secs(5),
        channel.bump(Context::default()),
    )
    .await
    .expect("the lane must still answer while a subscription is open")
    .unwrap();
    assert_eq!(n, 1);
    drop(ticks);
}

/// r[verify jetstream.subscription.cancel]
/// r[verify jetstream.subscription.surface.cancellation]
/// Dropping the subscription must stop the *producer*, not merely stop
/// the delivery.
#[tokio::test]
async fn dropping_a_generated_subscription_stops_the_producer() {
    let wired = Wired::new();
    let mut ticks = wired.lane().await.ticks(Context::default());
    let bumper = wired.lane().await;

    ticks.establish().await;
    until("the producer must start", || {
        wired.producers.load(SeqCst) == 1
    })
    .await;
    bumper.bump(Context::default()).await.unwrap();

    let first = tokio::time::timeout(Duration::from_secs(5), ticks.next())
        .await
        .expect("the first item must arrive");
    assert!(matches!(first, Some(Ok(Item::Next(Tick(1))))));

    drop(ticks);
    until("dropping the subscription must stop the producer", || {
        wired.producers.load(SeqCst) == 0
    })
    .await;
}

/// r[verify jetstream.subscription.surface.terminal-value]
/// r[verify jetstream.subscription.surface.composition]
/// The end is a value, it carries a result, and a merge keeps both the
/// result and which subscription it came from.
#[tokio::test]
async fn merging_keeps_the_result_and_says_whose() {
    let wired = Wired::new();
    let mut one = wired.lane().await.ticks(Context::default());
    let mut two = wired.lane().await.ticks(Context::default());
    let bumper = wired.lane().await;

    one.establish().await;
    two.establish().await;
    until("both producers must start", || {
        wired.producers.load(SeqCst) == 2
    })
    .await;

    bumper.bump(Context::default()).await.unwrap();
    bumper.bump(Context::default()).await.unwrap();

    let merged = subscription::merge([("one", one), ("two", two)]);
    let collecting = tokio::spawn(async move {
        let mut merged = merged;
        let mut ends = Vec::new();
        while let Some(next) = merged.next().await {
            match next {
                (who, Ok(Item::Done(Total(total)))) => ends.push((who, total)),
                (who, Err(e)) => panic!("{who} failed unexpectedly: {e}"),
                _ => {}
            }
        }
        ends
    });
    settle().await;
    wired.stop.cancel();

    let mut ends = tokio::time::timeout(Duration::from_secs(5), collecting)
        .await
        .expect("both subscriptions must end")
        .unwrap();
    ends.sort();
    assert_eq!(ends, vec![("one", 2), ("two", 2)]);
}

/// r[verify jetstream.subscription.compat.existing-clients]
/// The unary method is untouched by any of this.
#[tokio::test]
async fn unary_methods_are_unchanged() {
    let wired = Wired::new();
    let channel = wired.lane().await;
    assert_eq!(channel.bump(Context::default()).await.unwrap(), 1);
    assert_eq!(channel.bump(Context::default()).await.unwrap(), 2);
}

// ---------------------------------------------------------------------------
// The same protocol on a session that has exactly one lane.
//
// r[verify jetstream.subscription.realisation]
// r[verify jetstream.subscription.conformance.single-lane]
// Everything above opens a lane per subscription, which is what a
// `LaneSupport::Many` session licenses. On `LaneSupport::One` the same
// code has to tag-multiplex two subscriptions and the unary calls onto
// one lane — and the caller's types must not change, which is what makes
// the realisation opaque rather than merely reported.

/// Two subscriptions and a unary call, all on one lane.
#[tokio::test]
async fn one_lane_carries_two_subscriptions_and_a_call() {
    let wired = Wired::new();
    let channel = wired.lane().await;

    let mut first = channel.ticks(Context::default());
    let mut second = channel.ticks(Context::default());
    // Both on the wire before anything is bumped. Without this the
    // second one does not exist yet when the tick happens — a
    // subscription opens on first read, and reading them in sequence
    // opens them in sequence.
    first.establish().await;
    second.establish().await;

    let reading = tokio::spawn(async move {
        let (a, b) = futures::future::join(first.next(), second.next()).await;
        (a, b, first, second)
    });

    // The lane is serving two subscriptions; it must still answer.
    let n = tokio::time::timeout(
        Duration::from_secs(5),
        channel.bump(Context::default()),
    )
    .await
    .expect("one lane must carry a call alongside its subscriptions")
    .unwrap();
    assert_eq!(n, 1);

    let (a, b, first, second) =
        tokio::time::timeout(Duration::from_secs(5), reading)
            .await
            .expect("both subscriptions must receive the tick")
            .unwrap();
    assert!(matches!(a, Some(Ok(Item::Next(Tick(1))))));
    assert!(matches!(b, Some(Ok(Item::Next(Tick(1))))));
    until("both producers must be alive", || {
        wired.producers.load(SeqCst) == 2
    })
    .await;

    // r[verify jetstream.subscription.identity]
    // Cancelling one must not disturb the other, which share a lane and
    // are told apart only by their tags.
    drop(first);
    until("dropping one must stop exactly one producer", || {
        wired.producers.load(SeqCst) == 1
    })
    .await;

    let mut second = second;
    let reading = tokio::spawn(async move { second.next().await });
    settle().await;
    channel.bump(Context::default()).await.unwrap();
    let still = tokio::time::timeout(Duration::from_secs(5), reading)
        .await
        .expect("the surviving subscription must still receive")
        .unwrap();
    assert!(matches!(still, Some(Ok(Item::Next(Tick(2))))));
}

/// The same subscription over a session that reports one lane and
/// refuses a second — the caller's code is identical.
#[tokio::test]
async fn a_single_lane_session_serves_a_subscription() {
    use jetstream_rpc::session::{
        Capabilities, LaneSupport, SessionError, SingleLaneSession,
    };

    // One lane, borrowed from an in-process pair, then presented as the
    // *only* lane each side has.
    let pair = LocalSession::<CounterChannel>::pair();
    let client_lane = Session::<CounterChannel>::open_lane(&pair.client)
        .await
        .expect("the pair is open");
    let service_lane = Session::<CounterChannel>::accept_lane(&pair.server)
        .await
        .expect("the pair is open");

    let one = SingleLaneSession::<CounterChannel, _, _>::client(client_lane);
    let caps: Capabilities = Session::<CounterChannel>::capabilities(&one);
    assert_eq!(caps.lanes, LaneSupport::One);

    let counting = Counting {
        ticks: broadcast::channel(64).0,
        stop: subscription::CancellationToken::new(),
        producers: Default::default(),
        seen: Default::default(),
    };
    let producers = counting.producers.clone();
    tokio::spawn(async move {
        let mut service = CounterService { inner: counting };
        let _ = jetstream_rpc::server::run(&mut service, service_lane).await;
    });

    let lane = Session::<CounterChannel>::open_lane(&one)
        .await
        .expect("the one lane is available");
    let channel = CounterChannel::new(16, Box::new(lane));

    // And there is no second one. A subscription on this session shares
    // the lane with everything else, which the caller cannot see.
    assert!(matches!(
        Session::<CounterChannel>::open_lane(&one).await,
        Err(SessionError::LaneLimitReached)
    ));

    let mut ticks = channel.ticks(Context::default());
    ticks.establish().await;
    channel.bump(Context::default()).await.unwrap();

    let first = tokio::time::timeout(Duration::from_secs(5), ticks.next())
        .await
        .expect("a one-lane session must deliver a subscription");
    assert!(matches!(first, Some(Ok(Item::Next(Tick(1))))));

    // r[verify jetstream.subscription.cancel]
    // Cancellation travels the subscription's own lane — which here is
    // the only lane there is.
    drop(ticks);
    until(
        "cancellation must reach the producer on a shared lane",
        || producers.load(SeqCst) == 0,
    )
    .await;
}

/// r[verify jetstream.subscription.surface.cancellation]
/// A producer polled without a dispatcher still learns when its
/// subscriber goes.
///
/// Serving in process — calling the service directly and polling what it
/// returns — used to hand the producer a token nothing would ever
/// cancel. Dropping the subscription woke nothing, and because
/// `producing` runs its body in a spawned task, that task outlived the
/// receiver it was sending to.
#[tokio::test]
async fn an_in_process_producer_is_cancelled_when_dropped() {
    let counting = Counting {
        ticks: broadcast::channel(64).0,
        stop: subscription::CancellationToken::new(),
        producers: Default::default(),
        seen: Default::default(),
    };
    let producers = counting.producers.clone();

    // No dispatcher anywhere: the service is called directly.
    let mut ticks = counting.ticks(Context::default());
    assert!(ticks.next().now_or_never().is_none(), "it starts");
    until("the producer must start", || producers.load(SeqCst) == 1).await;

    drop(ticks);
    until(
        "dropping it must stop the producer, dispatcher or not",
        || producers.load(SeqCst) == 0,
    )
    .await;
}

/// r[verify jetstream.subscription.surface.cancellation]
/// The same promise, one step removed: a producer reached through
/// `opening` rather than polled directly.
///
/// Opening runs inside a boxed future that cannot reach the subscription
/// the caller holds, so the token it made for the producer had no owner
/// — dropping the subscription woke nothing, and the spawned `producing`
/// task went on sending to a receiver that was gone. The guard now
/// travels out of the opening future to the subscription itself.
#[tokio::test]
async fn a_producer_opened_into_is_cancelled_when_dropped() {
    let counting = Counting {
        ticks: broadcast::channel(64).0,
        stop: subscription::CancellationToken::new(),
        producers: Default::default(),
        seen: Default::default(),
    };
    let producers = counting.producers.clone();

    // The shape `opening` exists for: a plain `fn` handing back a
    // subscription whose real work is deferred, wrapping a producer.
    // A clone goes into the opener so `counting` — and with it the
    // broadcast sender the producer is reading — outlives the future;
    // otherwise the producer ends because its feed closed, which is not
    // what this test is about.
    let opener = counting.clone();
    let mut ticks =
        Subscription::opening(async move { opener.ticks(Context::default()) });

    // Open it, so the producer starts. Without opening first there is
    // nothing to leak.
    ticks.establish().await;
    until("the producer must start", || producers.load(SeqCst) == 1).await;

    drop(ticks);
    until(
        "dropping an opened subscription must stop its producer too",
        || producers.load(SeqCst) == 0,
    )
    .await;
}

// ---------------------------------------------------------------------
// A subscription whose producer fails partway through.
// ---------------------------------------------------------------------

#[service(uses(super::{Tick, Total}))]
pub trait Flaky {
    #[subscription]
    fn flaky(&self, ctx: Context) -> Subscription<Tick, Total>;
}

use crate::flaky_protocol::{FlakyChannel, FlakyService};

#[derive(Clone, Default)]
struct Failing {
    /// How many items the source was asked to produce. The whole point
    /// of the test: it must stop at the failure and not one item later.
    produced: Arc<AtomicUsize>,
}

impl Flaky for Failing {
    fn flaky(&self, _ctx: Context) -> Subscription<Tick, Total> {
        let produced = self.produced.clone();
        Subscription::served(move |_cancel| {
            futures::stream::unfold(0usize, move |n| {
                let produced = produced.clone();
                async move {
                    produced.fetch_add(1, SeqCst);
                    let item = match n {
                        0 => Ok(Item::Next(Tick(1))),
                        1 => Err(Error::new("the room burned down")),
                        // Only reachable if the failure was not
                        // terminal. Deliberately endless: a response
                        // stream that keeps polling never stops, which
                        // is the leak this guards.
                        _ => Ok(Item::Next(Tick(99))),
                    };
                    Some((item, n + 1))
                }
            })
        })
    }
}

/// The session has to outlive the lane, so it is handed back rather
/// than dropped at the end of the setup.
async fn flaky_lane(
) -> (LocalSession<FlakyChannel>, FlakyChannel, Arc<AtomicUsize>) {
    let pair = LocalSession::<FlakyChannel>::pair();
    let failing = Failing::default();
    let produced = failing.produced.clone();
    let server = pair.server.clone();
    tokio::spawn(async move {
        while let Ok(lane) = Session::<FlakyChannel>::accept_lane(&server).await
        {
            let inner = failing.clone();
            tokio::spawn(async move {
                let mut service = FlakyService { inner };
                let _ = jetstream_rpc::server::run(&mut service, lane).await;
            });
        }
    });
    let lane = Session::<FlakyChannel>::open_lane(&pair.client)
        .await
        .expect("the session is open");
    (pair.client, FlakyChannel::new(16, Box::new(lane)), produced)
}

/// r[verify jetstream.subscription.surface.termination]
/// A producer failure is an *ending*. The dispatcher frees the tag when
/// the response stream ends and nowhere else, so a failure that left the
/// stream running would hold the tag for the life of the connection and
/// deliver items after an error the subscriber had already seen.
#[tokio::test]
async fn a_producer_failure_ends_the_subscription() {
    let (_session, channel, produced) = flaky_lane().await;
    let mut items = channel.flaky(Context::default());

    let first = tokio::time::timeout(Duration::from_secs(5), items.next())
        .await
        .expect("the first item must arrive")
        .expect("the subscription must not end before it starts");
    assert_eq!(
        first.expect("the first item is not the failure"),
        Item::Next(Tick(1)),
    );

    // The failure itself reaches the subscriber, rather than tearing
    // down the lane the way a transport error would.
    let second = tokio::time::timeout(Duration::from_secs(5), items.next())
        .await
        .expect("the failure must arrive")
        .expect("the failure is an item, not the end of the stream");
    assert!(second.is_err(), "expected the failure, got {second:?}");

    // Give a source that kept running every chance to produce `Tick(99)`.
    settle().await;
    assert_eq!(
        produced.load(SeqCst),
        2,
        "the source must be polled for the item and the failure, and \
         then never again",
    );
}

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

use futures::StreamExt;
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

    // A subscription opens when it is first polled, so nothing is on the
    // wire yet — hence the settle after the first read below.
    let first = tokio::spawn(async move {
        let mut got = Vec::new();
        while let Some(item) = ticks.next().await {
            match item.unwrap() {
                Item::Next(Tick(n)) => {
                    got.push(n);
                    if got.len() == 3 {
                        break;
                    }
                }
                Item::Done(_) => break,
            }
        }
        got
    });
    settle().await;

    for _ in 0..3 {
        bumper.bump(Context::default()).await.unwrap();
    }

    let got = tokio::time::timeout(Duration::from_secs(5), first)
        .await
        .expect("three items must arrive")
        .unwrap();
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

    // Open it: nothing reaches the service until the stream is polled.
    let opened = tokio::spawn(async move { ticks.next().await });
    settle().await;

    let n = tokio::time::timeout(
        Duration::from_secs(5),
        channel.bump(Context::default()),
    )
    .await
    .expect("the lane must still answer while a subscription is open")
    .unwrap();
    assert_eq!(n, 1);
    opened.abort();
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

    let reading = tokio::spawn(async move {
        let item = ticks.next().await;
        // Hold it open until told otherwise.
        (item, ticks)
    });
    settle().await;
    bumper.bump(Context::default()).await.unwrap();

    let (first, held) = tokio::time::timeout(Duration::from_secs(5), reading)
        .await
        .expect("the first item must arrive")
        .unwrap();
    assert!(matches!(first, Some(Ok(Item::Next(Tick(1))))));
    until("the producer must start", || {
        wired.producers.load(SeqCst) == 1
    })
    .await;

    drop(held);
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
    let one = wired.lane().await.ticks(Context::default());
    let two = wired.lane().await.ticks(Context::default());
    let bumper = wired.lane().await;

    let merged = subscription::merge([("one", one), ("two", two)]);
    let collecting = tokio::spawn(async move {
        let mut merged = merged;
        let mut ends = Vec::new();
        while let Some(next) = merged.next().await {
            if let (who, Item::Done(Total(total))) = next.unwrap() {
                ends.push((who, total));
            }
        }
        ends
    });
    settle().await;

    bumper.bump(Context::default()).await.unwrap();
    bumper.bump(Context::default()).await.unwrap();
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

//! Subscriptions and sessions over a simulated network.
//!
//! `tests/subscriptions.rs` proves the promises in process, where the
//! transport is a channel and nothing can go wrong with it. These prove
//! the same promises over a socket that can be slow, partitioned, or cut
//! — which is where a held tag or an unheard cancellation actually costs
//! something, and where in-process tests are structurally unable to look.
//!
//! Turmoil makes that deterministic: simulated time, one thread, a
//! network whose faults are injected rather than waited for.

use std::{
    net::{IpAddr, Ipv4Addr},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering::SeqCst},
        Arc,
    },
    time::Duration,
};

use futures::StreamExt;
use jetstream::prelude::*;
use jetstream_rpc::session::{Session, SessionError, SingleLaneSession};
use turmoil::{
    net::{TcpListener, TcpStream},
    Builder,
};

use crate::feed_protocol::{FeedChannel, FeedService};

#[derive(Debug, Clone, PartialEq, Eq, JetStreamWireFormat)]
pub struct Tick(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, JetStreamWireFormat)]
pub struct Closed(pub u64);

#[service(uses(super::{Tick, Closed}))]
pub trait Feed {
    /// Unary, and it must keep working while a subscription is open on
    /// the same lane.
    async fn bump(&self, ctx: Context) -> Result<u64>;

    /// Streaming: a tick every 100ms until the subscriber goes.
    #[subscription]
    fn feed(&self, ctx: Context) -> Subscription<Tick, Closed>;
}

#[derive(Clone, Default)]
struct Ticker {
    /// How many ticks the producer has handed to the lane.
    sent: Arc<AtomicU64>,
    /// How many producers are alive — the only way to see that
    /// cancellation reached the *work* and not merely the delivery.
    producers: Arc<AtomicUsize>,
}

impl Feed for Ticker {
    async fn bump(&self, _ctx: Context) -> Result<u64> {
        Ok(self.sent.load(SeqCst))
    }

    fn feed(&self, _ctx: Context) -> Subscription<Tick, Closed> {
        let sent = self.sent.clone();
        let producers = self.producers.clone();
        Subscription::producing(8, move |producer| async move {
            producers.fetch_add(1, SeqCst);
            let mut n = 0u64;
            // No `is_cancelled`, no `cancelled().await`: the send is the
            // third of the three forms cancellation is offered in, and
            // over a network it is the one a producer actually reaches.
            while producer.send(Tick(n)).await.is_ok() {
                n += 1;
                sent.store(n, SeqCst);
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            producers.fetch_sub(1, SeqCst);
        })
    }
}

const PORT: u16 = 1739;

/// The server host: accept a connection, present it as this peer's one
/// lane, and serve it.
fn serve(sim: &mut turmoil::Sim<'_>, ticker: Ticker) {
    sim.host("server", move || {
        let ticker = ticker.clone();
        async move {
            let listener =
                TcpListener::bind((IpAddr::from(Ipv4Addr::UNSPECIFIED), PORT))
                    .await?;
            loop {
                let (stream, _) = listener.accept().await?;
                let ticker = ticker.clone();
                tokio::spawn(async move {
                    let session =
                        SingleLaneSession::<FeedChannel, _, _>::service_io(
                            stream,
                        );
                    let Ok(lane) =
                        Session::<FeedChannel>::accept_lane(&session).await
                    else {
                        return;
                    };
                    let mut service = FeedService { inner: ticker };
                    let _ =
                        jetstream_rpc::server::run(&mut service, lane).await;
                });
            }
        }
    });
}

/// The caller's side: connect, and present the connection as the one
/// lane this peer has.
async fn connect() -> turmoil::Result<impl Session<FeedChannel>> {
    let stream = TcpStream::connect(("server", PORT)).await?;
    Ok(SingleLaneSession::<FeedChannel, _, _>::client_io(stream))
}

/// r[verify jetstream.subscription.overview]
/// One request, many responses — over a socket, with the latency and
/// framing an in-process channel does not have.
#[test]
fn a_subscription_streams_over_a_real_socket() {
    let mut sim = Builder::new()
        .simulation_duration(Duration::from_secs(30))
        .build();
    serve(&mut sim, Ticker::default());

    sim.client("client", async move {
        let session = connect().await?;
        let lane = Session::<FeedChannel>::open_lane(&session)
            .await
            .expect("the one lane is available");
        let channel = FeedChannel::new(16, Box::new(lane));

        let mut feed = channel.feed(Context::default());
        feed.establish().await;

        let mut got = Vec::new();
        while got.len() < 4 {
            match feed.next().await {
                Some(Ok(Item::Next(Tick(n)))) => got.push(n),
                other => panic!("expected a tick, got {other:?}"),
            }
        }
        assert_eq!(got, vec![0, 1, 2, 3], "in order, and none dropped");
        Ok(())
    });

    sim.run().expect("the simulation must finish");
}

/// r[verify jetstream.subscription.dispatch.concurrent]
/// A subscription is one call among the lane's others — and here there
/// is only one lane, so a dispatcher that served the subscription by
/// consuming the lane could never answer the call.
#[test]
fn a_unary_call_shares_the_lane_with_a_live_subscription() {
    let mut sim = Builder::new()
        .simulation_duration(Duration::from_secs(30))
        .build();
    serve(&mut sim, Ticker::default());

    sim.client("client", async move {
        let session = connect().await?;
        let lane = Session::<FeedChannel>::open_lane(&session)
            .await
            .expect("the one lane is available");
        let channel = FeedChannel::new(16, Box::new(lane));

        let mut feed = channel.feed(Context::default());
        feed.establish().await;

        // Let the producer get chatty first: the failure this guards
        // against is a busy subscription starving everything else.
        for _ in 0..3 {
            assert!(matches!(feed.next().await, Some(Ok(Item::Next(_)))));
        }

        let answered = tokio::time::timeout(
            Duration::from_secs(5),
            channel.bump(Context::default()),
        )
        .await
        .expect("a live subscription must not stop the lane answering")
        .expect("the call itself must succeed");
        assert!(answered >= 3, "the call saw the producer's progress");
        Ok(())
    });

    sim.run().expect("the simulation must finish");
}

/// r[verify jetstream.subscription.surface.cancellation]
/// r[verify jetstream.subscription.cancel]
/// Dropping the subscription must stop the *producer*, across a network
/// — the cancellation has to be framed, sent, and acted on, none of
/// which an in-process test exercises.
#[test]
fn dropping_a_subscription_stops_the_producer_across_the_network() {
    let mut sim = Builder::new()
        .simulation_duration(Duration::from_secs(30))
        .build();
    let ticker = Ticker::default();
    let producers = ticker.producers.clone();
    serve(&mut sim, ticker);

    sim.client("client", async move {
        let session = connect().await?;
        let lane = Session::<FeedChannel>::open_lane(&session)
            .await
            .expect("the one lane is available");
        let channel = FeedChannel::new(16, Box::new(lane));

        let mut feed = channel.feed(Context::default());
        feed.establish().await;
        assert!(matches!(feed.next().await, Some(Ok(Item::Next(_)))));

        drop(feed);

        // The cancellation has a whole round trip to make. Give it one,
        // and fail loudly rather than hanging if it never arrives.
        let deadline = Duration::from_secs(10);
        let started = tokio::time::Instant::now();
        while producers.load(SeqCst) != 0 {
            assert!(
                started.elapsed() < deadline,
                "dropping a subscription must stop the producer at the \
                 other end of the connection",
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        Ok(())
    });

    sim.run().expect("the simulation must finish");
}

/// r[verify jetstream.subscription.surface.termination]
/// The connection dropping is one of the three outcomes a subscriber
/// must be able to tell apart, and the only one no amount of waiting
/// resolves. A subscription left in silence when its lane died is the
/// failure this guards: the caller waits forever for an item that cannot
/// come, holding a tag nothing will release.
#[test]
fn a_crashed_server_tells_its_subscriber() {
    let mut sim = Builder::new()
        .simulation_duration(Duration::from_secs(30))
        .build();
    serve(&mut sim, Ticker::default());

    // Set once the subscription is live, so the crash lands mid-stream
    // rather than before anything has been established.
    let streaming = Arc::new(AtomicBool::new(false));
    let flag = streaming.clone();

    sim.client("client", async move {
        let session = connect().await?;
        let lane = Session::<FeedChannel>::open_lane(&session)
            .await
            .expect("the one lane is available");
        let channel = FeedChannel::new(16, Box::new(lane));

        let mut feed = channel.feed(Context::default());
        feed.establish().await;
        assert!(matches!(feed.next().await, Some(Ok(Item::Next(_)))));
        flag.store(true, SeqCst);

        // Whatever comes next, it must not be silence. Either the
        // subscriber is told the lane failed, or the stream ends — both
        // are answers; hanging is not.
        let outcome = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                match feed.next().await {
                    // Items already in flight when the link died.
                    Some(Ok(Item::Next(_))) => continue,
                    other => break other,
                }
            }
        })
        .await
        .expect("a subscription whose lane died must not hang");

        // Not merely "something happened": the subscriber is *told*.
        // A bare end of stream would be the truncation this rule
        // forbids — indistinguishable from a subscription that finished.
        let failure = match outcome {
            Some(Err(e)) => e,
            other => panic!(
                "a lane that died must be reported, not left as silence \
                 or a clean end: {other:?}"
            ),
        };
        let message = failure.to_string();
        assert!(
            message.contains("lane"),
            "the reason must name what failed: {message}"
        );
        Ok(())
    });

    // Step until the subscription is live, then take the server away.
    while !streaming.load(SeqCst) {
        // `step` reports whether every client has *finished*, so a `true`
        // here means the client gave up before it ever streamed.
        let finished = sim.step().expect("the simulation must step");
        assert!(
            !finished,
            "the simulation ended before the subscription started",
        );
    }
    sim.crash("server");

    sim.run().expect("the simulation must finish");
}

/// r[verify jetstream.session.conformance.single-stream]
/// A framed byte stream is a session with exactly one lane. Over a real
/// socket the second `open_lane` must still be refused rather than
/// quietly handing back a second view of the same connection.
#[test]
fn a_single_lane_session_over_tcp_refuses_a_second_lane() {
    let mut sim = Builder::new()
        .simulation_duration(Duration::from_secs(30))
        .build();
    serve(&mut sim, Ticker::default());

    sim.client("client", async move {
        let session = connect().await?;

        // r[verify jetstream.session.capabilities]
        let caps = Session::<FeedChannel>::capabilities(&session);
        assert_eq!(
            caps.lanes,
            jetstream_rpc::session::LaneSupport::One,
            "a byte stream is one lane, and says so",
        );

        let lane = Session::<FeedChannel>::open_lane(&session)
            .await
            .expect("the one lane is available");

        assert!(
            matches!(
                Session::<FeedChannel>::open_lane(&session).await,
                Err(SessionError::LaneLimitReached)
            ),
            "a second lane on a byte stream must be refused",
        );

        // And the one lane still works, which is the point of refusing
        // rather than failing the session.
        let channel = FeedChannel::new(16, Box::new(lane));
        let mut feed = channel.feed(Context::default());
        feed.establish().await;
        assert!(matches!(feed.next().await, Some(Ok(Item::Next(Tick(0))))));
        Ok(())
    });

    sim.run().expect("the simulation must finish");
}

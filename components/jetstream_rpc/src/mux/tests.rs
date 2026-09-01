//! The client half of a subscription, over an in-memory transport.
//!
//! A hand-rolled protocol whose response type has a terminator, so the
//! routing can be tested before the `#[service]` macro generates one.
use std::{
    io,
    pin::Pin,
    task::{Context as Cx, Poll},
};

use futures::{channel::mpsc as fmpsc, Sink, SinkExt, Stream, StreamExt};
use jetstream_wireformat::WireFormat;

use tokio::sync::mpsc;

use crate::{
    client::ClientTransport, context::Context, mux::Waiter,
    subscription::RDONE, Error, Frame, Framer, Mux, Protocol,
};

const TASK: u8 = 102;
const RITEM: u8 = 103;

#[derive(Debug, PartialEq)]
pub struct Ask(pub u32);

#[derive(Debug, PartialEq)]
pub enum Say {
    Item(u32),
    Done,
}

impl Framer for Ask {
    fn message_type(&self) -> u8 {
        TASK
    }

    fn byte_size(&self) -> u32 {
        4
    }

    fn encode<W: io::Write>(&self, w: &mut W) -> io::Result<()> {
        WireFormat::encode(&self.0, w)
    }

    fn decode<R: io::Read>(r: &mut R, _ty: u8) -> io::Result<Self> {
        Ok(Ask(WireFormat::decode(r)?))
    }
}

impl Framer for Say {
    // r[impl jetstream.subscription.termination]
    // The end is a value in the sequence, not the absence of one — which
    // is what lets it survive a merge and carry a payload later.
    fn message_type(&self) -> u8 {
        match self {
            Say::Item(_) => RITEM,
            Say::Done => RDONE,
        }
    }

    fn byte_size(&self) -> u32 {
        match self {
            Say::Item(_) => 4,
            Say::Done => 0,
        }
    }

    fn encode<W: io::Write>(&self, w: &mut W) -> io::Result<()> {
        match self {
            Say::Item(n) => WireFormat::encode(n, w),
            Say::Done => Ok(()),
        }
    }

    fn decode<R: io::Read>(r: &mut R, ty: u8) -> io::Result<Self> {
        match ty {
            RITEM => Ok(Say::Item(WireFormat::decode(r)?)),
            RDONE => Ok(Say::Done),
            other => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown message type: {other}"),
            )),
        }
    }
}

pub struct Counting;
impl Protocol for Counting {
    type Error = Error;
    type Request = Ask;
    type Response = Say;

    const NAME: &'static str = "counting";
    const VERSION: &'static str = "rs.jetstream.proto/counting/0.0.0-test";
}

struct Duplex {
    tx: fmpsc::UnboundedSender<Frame<Ask>>,
    rx: fmpsc::UnboundedReceiver<Result<Frame<Say>, Error>>,
}

impl Sink<Frame<Ask>> for Duplex {
    type Error = Error;

    fn poll_ready(
        mut self: Pin<&mut Self>,
        cx: &mut Cx<'_>,
    ) -> Poll<Result<(), Error>> {
        Pin::new(&mut self.tx)
            .poll_ready(cx)
            .map_err(|e| Error::new(e.to_string()))
    }

    fn start_send(
        mut self: Pin<&mut Self>,
        item: Frame<Ask>,
    ) -> Result<(), Error> {
        Pin::new(&mut self.tx)
            .start_send(item)
            .map_err(|e| Error::new(e.to_string()))
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Cx<'_>,
    ) -> Poll<Result<(), Error>> {
        Pin::new(&mut self.tx)
            .poll_flush(cx)
            .map_err(|e| Error::new(e.to_string()))
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        cx: &mut Cx<'_>,
    ) -> Poll<Result<(), Error>> {
        Pin::new(&mut self.tx)
            .poll_close(cx)
            .map_err(|e| Error::new(e.to_string()))
    }
}

impl Stream for Duplex {
    type Item = Result<Frame<Say>, Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Cx<'_>,
    ) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.rx).poll_next(cx)
    }
}

type Peer = fmpsc::UnboundedSender<Result<Frame<Say>, Error>>;

/// A producer that answers one `Ask(n)` with `n` items and a terminator.
/// The second return is the peer's own sender, so a test can emit a frame
/// no caller solicited — down the real demultiplexer, not around it.
fn counting_transport() -> (Box<dyn ClientTransport<Counting>>, Peer) {
    let (to_server, mut from_client) = fmpsc::unbounded();
    let (to_client, from_server) = fmpsc::unbounded();
    let peer = to_client.clone();
    tokio::spawn(async move {
        while let Some(Frame { tag, msg: Ask(n) }) = from_client.next().await {
            let mut out = to_client.clone();
            // `Ask(0)` is a subscription that stays open: no items and no
            // terminator, which is the shape every use case actually has
            // and the only one in which a lane failure has anything to
            // resolve.
            if n == 0 {
                continue;
            }
            for i in 0..n {
                let item = Frame {
                    tag,
                    msg: Say::Item(i),
                };
                if out.send(Ok(item)).await.is_err() {
                    return;
                }
            }
            let _ = out
                .send(Ok(Frame {
                    tag,
                    msg: Say::Done,
                }))
                .await;
        }
    });
    (
        Box::new(Duplex {
            tx: to_server,
            rx: from_server,
        }),
        peer,
    )
}

/// r[impl jetstream.subscription.overview]
/// One request, many responses, terminated explicitly.
#[tokio::test]
async fn a_streaming_call_yields_many_responses_under_one_tag() {
    let (transport, _peer) = counting_transport();
    let mux = Mux::<Counting>::new(4, transport);
    let mut items = mux.rpc_stream(Context::default(), Ask(5), 16).await;
    let tag = items.tag;

    let mut got = Vec::new();
    while let Some(frame) = items.next().await {
        let frame = frame.unwrap();
        // r[impl jetstream.subscription.definition]
        // Every response shares the request's tag: the tag *is* the
        // subscription for as long as it is in flight.
        assert_eq!(frame.tag, tag);
        match frame.msg {
            Say::Item(n) => got.push(n),
            Say::Done => break,
        }
    }
    assert_eq!(got, vec![0, 1, 2, 3, 4]);
}
/// r[impl jetstream.rcp.multiplexing]
/// r[impl jetstream.subscription.surface.termination]
/// A frame for a tag nobody holds is a **lane protocol error**, and
/// every waiter on that lane is told.
///
/// Three behaviours in three revisions, and the middle one was the worst.
/// It began as `in_flight.remove(&tag).unwrap()`, so any unsolicited or
/// duplicate frame panicked the demultiplexer and took the client with
/// it. That became log-and-continue, which stops the panic and leaves
/// the real problem: the stray frame proves this end and the peer
/// disagree about what is in flight, and carrying on leaves the tag
/// eligible for reuse — so the *next* stray frame bearing it is
/// delivered to whichever call has since been bound to it. A detected
/// desynchronisation becomes a silent response misbinding.
///
/// So the lane fails, and — the part that matters for a subscription —
/// its waiters find out. A streaming waiter left in the map keeps its
/// receiver open, because `RpcStream` holds the same map, so its caller
/// would otherwise wait forever for an item that cannot come.
#[tokio::test]
async fn an_unknown_tag_fails_the_lane_and_wakes_its_waiters() {
    let (transport, peer) = counting_transport();
    let mux = Mux::<Counting>::new(4, transport);

    // A subscription that stays open, which is the case that has
    // anything to lose: one that has already ended has been resolved.
    let mut items = mux.rpc_stream(Context::default(), Ask(0), 8).await;

    // Nobody ever issued tag 9.
    peer.unbounded_send(Ok(Frame {
        tag: 9,
        msg: Say::Item(1),
    }))
    .unwrap();

    // The subscription must be resolved, not left hanging. Before the
    // drain existed this timed out: the sender stayed in the map, so the
    // receiver never closed.
    let ended =
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while let Some(next) = items.next().await {
                if next.is_err() {
                    return Some(next);
                }
            }
            None
        })
        .await
        .expect("a failed lane must resolve its subscriptions, not hang");
    let err = ended.expect("the subscription ends with an error, not silence");
    let message = err.unwrap_err().to_string();
    assert!(
        message.contains("lane failed") || message.contains("lane closed"),
        "the reason must say the lane went, not just that it stopped: \
         {message}"
    );
}

/// r[impl jetstream.subscription.backpressure]
/// r[impl jetstream.subscription.backpressure.reporting]
/// One subscriber that stops reading must not stop the lane.
///
/// The demultiplexer is the lane's only reader. Awaiting a subscriber's
/// bounded channel there means the transport's receive window is
/// consumed by whichever subscriber stopped polling, and every other
/// subscription and every unary call on that lane waits for it — which
/// is the opposite of what `r[jetstream.subscription.fanout]` promises a
/// room. The specification allows exactly two responses to a subscriber
/// that cannot keep up, and stalling its neighbours is neither.
#[tokio::test]
async fn a_subscriber_that_stops_reading_does_not_stall_the_lane() {
    let (transport, _peer) = counting_transport();
    let mux = Mux::<Counting>::new(4, transport);

    // Capacity one, and nothing ever reads it. The peer sends far more.
    let lagging = mux.rpc_stream(Context::default(), Ask(64), 1).await;

    // The lane must still serve somebody else, promptly. Before this
    // fix the demultiplexer was parked awaiting the channel above and
    // this timed out.
    let mut healthy = mux.rpc_stream(Context::default(), Ask(2), 8).await;
    let first =
        tokio::time::timeout(std::time::Duration::from_secs(5), healthy.next())
            .await
            .expect(
                "a lagging subscriber must not block the lane's other calls",
            );
    assert!(matches!(first, Some(Ok(_))));

    // And the one that fell behind is told, rather than silently
    // truncated: `surface.termination` requires a gap not to look like a
    // normal end.
    let mut lagging = lagging;
    let outcome =
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while let Some(next) = lagging.next().await {
                if next.is_err() {
                    return next.err();
                }
            }
            None
        })
        .await
        .expect("the lagging subscription must resolve");
    let message = outcome
        .expect("a terminated subscriber is told why, not just cut off")
        .to_string();
    assert!(
        message.contains("fell behind"),
        "the reason must name the cause: {message}"
    );
}

/// A transport that accepts requests and answers nothing, so a test can
/// drive the peer by hand. Dropping the returned `Peer` closes the lane.
fn quiet_transport() -> (Box<dyn ClientTransport<Counting>>, Peer) {
    let (to_server, mut from_client) = fmpsc::unbounded();
    let (to_client, from_server) = fmpsc::unbounded();
    tokio::spawn(async move { while from_client.next().await.is_some() {} });
    (
        Box::new(Duplex {
            tx: to_server,
            rx: from_server,
        }),
        to_client,
    )
}

async fn settle() {
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
}

/// r[verify jetstream.subscription.identity]
/// A subscriber that fills its queue at the very moment the terminator
/// arrives must still release its tag. `ends_here` has already taken the
/// waiter out of the map, so marking it abandoned puts it back — and
/// nothing further will ever arrive under that tag to take it out again.
#[tokio::test]
async fn overflowing_on_the_terminator_still_releases_the_tag() {
    let (transport, _peer) = counting_transport();
    let mux = Mux::<Counting>::new(4, transport);

    // Capacity one, never read: `Ask(1)` sends a single item, which
    // fills the queue exactly, and then the terminator, which cannot fit.
    let lagging = mux.rpc_stream(Context::default(), Ask(1), 1).await;
    let tag = lagging.tag;
    settle().await;

    assert!(
        !mux.in_flight.lock().await.contains_key(&tag),
        "the terminator freed the tag, so nothing may remain in flight \
         under it — an abandoned entry here is never collected",
    );
}

/// r[verify jetstream.subscription.surface.termination]
/// The shutdown drain has the same obligation as the delivery loop: it
/// must not await a subscriber. Waiters are resolved in tag order, so one
/// full queue at the front would leave everything behind it unresolved
/// and every one of those tags unreleased.
#[tokio::test]
async fn a_full_subscriber_does_not_block_the_shutdown_drain() {
    let (transport, mut peer) = quiet_transport();
    let mux = Mux::<Counting>::new(4, transport);

    // Held, never read, and filled to capacity — but not *over* it, so
    // it is still a live streaming waiter rather than an abandoned one.
    let stuck = mux.rpc_stream(Context::default(), Ask(0), 1).await;
    peer.send(Ok(Frame {
        tag: stuck.tag,
        msg: Say::Item(0),
    }))
    .await
    .unwrap();
    settle().await;

    // Behind it in tag order, and therefore behind it in the drain.
    let mut waiting = mux.rpc_stream(Context::default(), Ask(0), 8).await;
    assert!(
        stuck.tag < waiting.tag,
        "the test needs the stuck waiter to be drained first",
    );

    drop(peer);

    let outcome =
        tokio::time::timeout(std::time::Duration::from_secs(5), waiting.next())
            .await
            .expect("a full subscriber must not hold up the drain behind it");
    assert!(
        matches!(outcome, Some(Err(_))),
        "the lane closed, and every waiter is owed that news: {outcome:?}",
    );
}

/// A transport whose outbound half is broken. The first frame the mux
/// task takes off the queue fails to send, which ends that task — and
/// from then on queuing a request fails.
struct Broken {
    rx: fmpsc::UnboundedReceiver<Result<Frame<Say>, Error>>,
}

impl Sink<Frame<Ask>> for Broken {
    type Error = Error;

    fn poll_ready(
        self: Pin<&mut Self>,
        _cx: &mut Cx<'_>,
    ) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(
        self: Pin<&mut Self>,
        _item: Frame<Ask>,
    ) -> Result<(), Error> {
        Err(Error::new("the transport is broken"))
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut Cx<'_>,
    ) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: Pin<&mut Self>,
        _cx: &mut Cx<'_>,
    ) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}

impl Stream for Broken {
    type Item = Result<Frame<Say>, Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Cx<'_>,
    ) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.rx).poll_next(cx)
    }
}

/// r[verify jetstream.subscription.surface.termination]
/// A subscription that never reached the lane must not read as one that
/// ended cleanly. Closing the item channel makes the first poll yield
/// `None`, which is exactly what an empty, successful subscription
/// yields — so the failure has to be carried explicitly.
#[tokio::test]
async fn a_subscription_that_was_never_issued_reports_it() {
    let (_to_client, from_server) = fmpsc::unbounded();
    let mux = Mux::<Counting>::new(4, Box::new(Broken { rx: from_server }));

    // The first request queues fine, then fails to send, which ends the
    // outbound task and closes the queue behind it.
    let _doomed = mux.rpc_stream(Context::default(), Ask(0), 8).await;
    settle().await;

    let mut never = mux.rpc_stream(Context::default(), Ask(0), 8).await;
    let first =
        tokio::time::timeout(std::time::Duration::from_secs(5), never.next())
            .await
            .expect("the failure must resolve rather than hang");
    let err = match first {
        Some(Err(e)) => e,
        other => panic!(
            "an unissued subscription must report a failure, not a clean \
             end: {other:?}"
        ),
    };
    assert!(
        err.to_string().contains("never issued"),
        "the reason must say what happened: {err}",
    );
}

/// r[verify jetstream.subscription.identity]
/// Dropping a finished stream must not disturb whoever holds its tag
/// now. The terminator released the number back to the pool, so the slot
/// may already belong to a different subscription — identity is the
/// binding, not the tag.
#[tokio::test]
async fn dropping_a_finished_stream_leaves_a_reused_tag_alone() {
    let (transport, _peer) = counting_transport();
    let mux = Mux::<Counting>::new(4, transport);

    let mut done = mux.rpc_stream(Context::default(), Ask(1), 8).await;
    let tag = done.tag;
    while let Some(item) = done.next().await {
        if matches!(item, Ok(Frame { msg: Say::Done, .. })) {
            break;
        }
    }
    settle().await;
    assert!(
        !mux.in_flight.lock().await.contains_key(&tag),
        "the terminator must have freed the tag",
    );

    // Whoever takes the tag next. Constructed directly: which numbers the
    // pool hands back is its business, and the hazard does not depend on
    // the reuse being immediate.
    let (tx, _rx) = mpsc::channel(4);
    mux.in_flight
        .lock()
        .await
        .insert(tag, Waiter::Streaming { binding: 9999, tx });

    drop(done);
    settle().await;

    assert!(
        matches!(
            mux.in_flight.lock().await.get(&tag),
            Some(Waiter::Streaming { binding: 9999, .. })
        ),
        "a stale stream abandoned a tag it no longer owned, silencing the \
         subscription that had taken it",
    );
}

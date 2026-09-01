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
    client::ClientTransport,
    context::Context,
    mux::Waiter,
    subscription::{Rcancel, Tcancel, RCANCEL, RDONE, TCANCEL},
    Error, Frame, Framer, Mux, Protocol,
};

const TASK: u8 = 102;
const RITEM: u8 = 103;

#[derive(Debug, PartialEq)]
pub enum Ask {
    /// Open a subscription. `Task(0)` is the *quiet* room: it emits
    /// nothing and stays open until something stops it, which is the
    /// shape every long-lived subscription actually has and the one a
    /// producer that merely counts down never exercises.
    Task(u32),
    /// r[impl jetstream.subscription.cancel]
    /// Cancellation is a request on the lane, under a fresh tag, naming
    /// its target in the payload.
    Cancel(Tcancel),
}

#[derive(Debug, PartialEq)]
pub enum Say {
    Item(u32),
    Done,
    Ack(Rcancel),
}

impl Framer for Ask {
    fn message_type(&self) -> u8 {
        match self {
            Ask::Task(_) => TASK,
            Ask::Cancel(_) => TCANCEL,
        }
    }

    fn byte_size(&self) -> u32 {
        match self {
            Ask::Task(_) => 4,
            Ask::Cancel(c) => WireFormat::byte_size(c),
        }
    }

    fn encode<W: io::Write>(&self, w: &mut W) -> io::Result<()> {
        match self {
            Ask::Task(n) => WireFormat::encode(n, w),
            Ask::Cancel(c) => WireFormat::encode(c, w),
        }
    }

    fn decode<R: io::Read>(r: &mut R, ty: u8) -> io::Result<Self> {
        match ty {
            TASK => Ok(Ask::Task(WireFormat::decode(r)?)),
            TCANCEL => Ok(Ask::Cancel(WireFormat::decode(r)?)),
            other => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown message type: {other}"),
            )),
        }
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
            Say::Ack(_) => RCANCEL,
        }
    }

    fn byte_size(&self) -> u32 {
        match self {
            Say::Item(_) => 4,
            Say::Done => 0,
            Say::Ack(a) => WireFormat::byte_size(a),
        }
    }

    fn encode<W: io::Write>(&self, w: &mut W) -> io::Result<()> {
        match self {
            Say::Item(n) => WireFormat::encode(n, w),
            Say::Done => Ok(()),
            Say::Ack(a) => WireFormat::encode(a, w),
        }
    }

    fn decode<R: io::Read>(r: &mut R, ty: u8) -> io::Result<Self> {
        match ty {
            RITEM => Ok(Say::Item(WireFormat::decode(r)?)),
            RDONE => Ok(Say::Done),
            RCANCEL => Ok(Say::Ack(WireFormat::decode(r)?)),
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
    tx: fmpsc::UnboundedSender<Result<Frame<Ask>, Error>>,
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
            .start_send(Ok(item))
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
        while let Some(Ok(Frame {
            tag,
            msg: Ask::Task(n),
        })) = from_client.next().await
        {
            let mut out = to_client.clone();
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
    let mut items = mux.rpc_stream(Context::default(), Ask::Task(5), 16).await;
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
            Say::Done | Say::Ack(_) => break,
        }
    }
    assert_eq!(got, vec![0, 1, 2, 3, 4]);
}

/// r[impl jetstream.subscription.compat.existing-clients]
/// Unary calls are untouched, and their tag is freed by the one response.
#[tokio::test]
async fn unary_calls_are_unaffected() {
    let (transport, _peer) = counting_transport();
    let mux = Mux::<Counting>::new(4, transport);
    let frame = mux
        .rpc(Context::default(), Ask::Task(1))
        .await
        .await
        .unwrap();
    assert_eq!(frame.msg, Say::Item(0));
}

/// r[impl jetstream.rcp.multiplexing]
/// r[impl jetstream.subscription.surface.termination]
/// A frame for a tag nobody holds is a **lane protocol error**, and
/// every waiter on that lane is told.
///
/// Three behaviours in three revisions, and the middle one was the
/// worst. It began as `in_flight.remove(&tag).unwrap()`, so any
/// unsolicited or duplicate frame panicked the demultiplexer and took
/// the client with it. That became log-and-continue, which stops the
/// panic and leaves the real problem: the stray frame proves this end
/// and the peer disagree about what is in flight, and carrying on leaves
/// the tag eligible for reuse — so the *next* stray frame bearing it is
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

    // The quiet room: open, and with everything still to lose. A
    // subscription that has already ended has been resolved already.
    let mut items = mux.rpc_stream(Context::default(), Ask::Task(0), 8).await;

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
/// subscription and every unary call on that lane waits for it — the
/// opposite of what `r[jetstream.subscription.fanout]` promises a room.
/// The specification allows exactly two responses to a subscriber that
/// cannot keep up, and stalling its neighbours is neither.
#[tokio::test]
async fn a_subscriber_that_stops_reading_does_not_stall_the_lane() {
    let (transport, _peer) = counting_transport();
    let mux = Mux::<Counting>::new(4, transport);

    // Capacity one, and nothing ever reads it. The peer sends far more.
    let lagging = mux.rpc_stream(Context::default(), Ask::Task(64), 1).await;

    // The lane must still serve somebody else, promptly. Before this fix
    // the demultiplexer was parked awaiting the channel above and this
    // timed out.
    let mut healthy = mux.rpc_stream(Context::default(), Ask::Task(2), 8).await;
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

// ---------------------------------------------------------------------------
// The server half, and then both halves against each other.

/// A room that answers `Ask(n)` with `n` items and a terminator, and
/// notices when the subscriber goes away.
#[derive(Clone)]
struct Room {
    /// Set when the producer observed cancellation, which is what
    /// `r[jetstream.subscription.cancel]` requires and what a `Sender`
    /// alone could never provide.
    stopped: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Protocol for Room {
    type Error = Error;
    type Request = Ask;
    type Response = Say;

    const NAME: &'static str = "room";
    const VERSION: &'static str = "rs.jetstream.proto/room/0.0.0-test";

    // r[impl jetstream.subscription.cancel]
    fn tcancel(oldtag: u16, binding: u64) -> Option<Ask> {
        Some(Ask::Cancel(Tcancel { oldtag, binding }))
    }
}

impl crate::server::Server for Room {
    // r[impl jetstream.subscription.surface.declared]
    fn is_streaming(message_type: u8) -> bool {
        message_type == TASK
    }

    // r[impl jetstream.subscription.cancel]
    fn cancel_target(frame: &Frame<Ask>) -> Option<u16> {
        match &frame.msg {
            Ask::Cancel(c) => Some(c.oldtag),
            _ => None,
        }
    }

    // r[impl jetstream.subscription.cancel]
    fn cancel_ack(oldtag: u16) -> Option<Say> {
        Some(Say::Ack(Rcancel { oldtag }))
    }

    // r[impl jetstream.subscription.termination]
    fn cancelled_terminator(method: u8) -> Option<Say> {
        assert_eq!(method, TASK, "the terminator must know its method");
        Some(Say::Done)
    }

    async fn rpc(
        &mut self,
        _ctx: Context,
        _frame: Frame<Ask>,
    ) -> Result<Frame<Say>, Error> {
        Err(Error::new("this protocol has only a streaming method"))
    }

    async fn rpc_stream(
        &mut self,
        _ctx: Context,
        frame: Frame<Ask>,
        cancel: tokio_util::sync::CancellationToken,
    ) -> crate::server::ResponseStream<Self> {
        let tag = frame.tag;
        let n = match frame.msg {
            Ask::Task(n) => n,
            Ask::Cancel(_) => 0,
        };
        let stopped = self.stopped.clone();
        // Note the asymmetry with the trait's *default* body, which must
        // be a bare `async move { .. }` block: `#[trait_variant::make]`
        // rewrites the trait, not its implementations, so here an
        // ordinary `async fn` body is exactly right.
        Box::pin(async_stream_items(tag, n, cancel, stopped))
    }
}

fn async_stream_items(
    tag: u16,
    n: u32,
    cancel: tokio_util::sync::CancellationToken,
    stopped: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> impl Stream<Item = Result<Frame<Say>, Error>> + Send {
    futures::stream::unfold(0u32, move |i| {
        let cancel = cancel.clone();
        let stopped = stopped.clone();
        async move {
            // r[impl jetstream.subscription.cancel]
            // The producer watches for cancellation between items, which
            // is the whole point of it being a parameter.
            if cancel.is_cancelled() {
                stopped.store(true, std::sync::atomic::Ordering::SeqCst);
                return None;
            }
            if n == 0 {
                // The quiet room. It has nothing to say until something
                // stops it, which is the shape a chat room, a build log
                // between lines, or a presence feed actually has.
                cancel.cancelled().await;
                stopped.store(true, std::sync::atomic::Ordering::SeqCst);
                return None;
            }
            if i < n {
                Some((
                    Ok(Frame {
                        tag,
                        msg: Say::Item(i),
                    }),
                    i + 1,
                ))
            } else if i == n {
                Some((
                    Ok(Frame {
                        tag,
                        msg: Say::Done,
                    }),
                    i + 1,
                ))
            } else {
                None
            }
        }
    })
}

/// r[impl jetstream.subscription.overview]
/// The server half: one request in, many responses out, terminated.
#[tokio::test]
async fn a_service_serves_a_subscription() {
    use crate::server::Server;

    let mut room = Room {
        stopped: Default::default(),
    };
    let cancel = tokio_util::sync::CancellationToken::new();
    let items = room
        .rpc_stream(
            Context::default(),
            Frame {
                tag: 3,
                msg: Ask::Task(4),
            },
            cancel,
        )
        .await;

    let got: Vec<Say> = items.map(|i| i.unwrap().msg).collect().await;
    assert_eq!(
        got,
        vec![
            Say::Item(0),
            Say::Item(1),
            Say::Item(2),
            Say::Item(3),
            Say::Done
        ]
    );
}

/// r[impl jetstream.subscription.cancel]
/// Cancellation reaches the *work*, not just the delivery. This is the
/// finding that a `Sender`-only producer surface could not satisfy: the
/// room stops producing rather than producing into a void.
#[tokio::test]
async fn cancellation_reaches_the_producer() {
    use crate::server::Server;

    let mut room = Room {
        stopped: Default::default(),
    };
    let stopped = room.stopped.clone();
    let cancel = tokio_util::sync::CancellationToken::new();

    let mut items = room
        .rpc_stream(
            Context::default(),
            Frame {
                tag: 1,
                msg: Ask::Task(1_000_000),
            },
            cancel.clone(),
        )
        .await;

    assert_eq!(items.next().await.unwrap().unwrap().msg, Say::Item(0));
    assert!(!stopped.load(std::sync::atomic::Ordering::SeqCst));

    cancel.cancel();
    assert!(
        items.next().await.is_none(),
        "a cancelled producer stops emitting"
    );
    assert!(
        stopped.load(std::sync::atomic::Ordering::SeqCst),
        "the producer must observe cancellation, not merely be ignored"
    );
}

/// r[impl jetstream.subscription.compat.existing-clients]
/// `is_streaming` defaults to false, so a protocol written before this
/// existed routes every request to `rpc` exactly as it did.
#[tokio::test]
async fn a_protocol_without_streaming_methods_is_unchanged() {
    assert!(!<Counting as crate::server::Server>::is_streaming(TASK));
}

impl crate::server::Server for Counting {
    async fn rpc(
        &mut self,
        _ctx: Context,
        frame: Frame<Ask>,
    ) -> Result<Frame<Say>, Error> {
        Ok(Frame {
            tag: frame.tag,
            msg: Say::Item(match frame.msg {
                Ask::Task(n) => n,
                Ask::Cancel(_) => 0,
            }),
        })
    }
}

/// The service side of a duplex, so `server::run` can be driven.
struct ServiceSide {
    tx: fmpsc::UnboundedSender<Result<Frame<Say>, Error>>,
    rx: fmpsc::UnboundedReceiver<Result<Frame<Ask>, Error>>,
}
impl Sink<Frame<Say>> for ServiceSide {
    type Error = Error;

    fn poll_ready(
        self: Pin<&mut Self>,
        _cx: &mut Cx<'_>,
    ) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Frame<Say>) -> Result<(), Error> {
        self.tx
            .unbounded_send(Ok(item))
            .map_err(|e| Error::new(e.to_string()))
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
impl Stream for ServiceSide {
    type Item = Result<Frame<Ask>, Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Cx<'_>,
    ) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.rx).poll_next(cx)
    }
}
impl crate::context::Contextual for ServiceSide {
    fn context(&self) -> Context {
        Context::default()
    }
}

/// r[impl jetstream.subscription.surface.declared]
/// The dispatcher routes on the declared message type. Without that
/// routing every request goes to `rpc`, which this protocol answers with
/// an error — so this test is what holds `run`'s streaming branch honest.
#[tokio::test]
async fn the_dispatcher_routes_a_streaming_request() {
    let (to_service, service_rx) = fmpsc::unbounded();
    let (service_tx, mut from_service) = fmpsc::unbounded();

    tokio::spawn(async move {
        let mut room = Room {
            stopped: Default::default(),
        };
        let _ = crate::server::run(
            &mut room,
            ServiceSide {
                tx: service_tx,
                rx: service_rx,
            },
        )
        .await;
    });

    to_service
        .unbounded_send(Ok(Frame {
            tag: 2,
            msg: Ask::Task(3),
        }))
        .unwrap();

    let mut got = Vec::new();
    while let Some(Ok(frame)) = from_service.next().await {
        assert_eq!(frame.tag, 2, "every response shares the request's tag");
        let done = frame.msg == Say::Done;
        got.push(frame.msg);
        if done {
            break;
        }
    }
    assert_eq!(
        got,
        vec![Say::Item(0), Say::Item(1), Say::Item(2), Say::Done],
        "the streaming branch must serve many responses, not one error"
    );
}

// ---------------------------------------------------------------------------
// Both halves against each other, which is where the dispatcher's own
// shape starts to matter.

/// A client whose service is `server::run` driving a `Room`, over an
/// in-memory duplex. The flag is the room's: set when the producer
/// observed cancellation.
fn wired() -> (Mux<Room>, std::sync::Arc<std::sync::atomic::AtomicBool>) {
    wired_with(16)
}

fn wired_with(
    tags: u16,
) -> (Mux<Room>, std::sync::Arc<std::sync::atomic::AtomicBool>) {
    let (to_service, service_rx) = fmpsc::unbounded();
    let (service_tx, from_service) = fmpsc::unbounded();
    let stopped: std::sync::Arc<std::sync::atomic::AtomicBool> =
        Default::default();
    let theirs = stopped.clone();
    tokio::spawn(async move {
        let mut room = Room { stopped: theirs };
        let _ = crate::server::run(
            &mut room,
            ServiceSide {
                tx: service_tx,
                rx: service_rx,
            },
        )
        .await;
    });
    let transport = Duplex {
        tx: to_service,
        rx: from_service,
    };
    (Mux::<Room>::new(tags, Box::new(transport)), stopped)
}

/// Collect a subscription's items up to its terminator, with a deadline
/// so a dispatcher that stopped serving fails the test instead of hanging
/// it.
async fn drain(items: &mut crate::RpcStream<Room>, what: &str) -> Vec<u32> {
    let mut got = Vec::new();
    loop {
        let next = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            items.next(),
        )
        .await
        .unwrap_or_else(|_| panic!("{what}"));
        match next {
            Some(Ok(Frame {
                msg: Say::Item(n), ..
            })) => got.push(n),
            _ => return got,
        }
    }
}

/// r[impl jetstream.subscription.definition]
/// A subscription is one call among the lane's others, not a takeover of
/// it. `r[jetstream.subscription.realisation.opaque]` lets an
/// implementation put many subscriptions on one lane — which it can only
/// do if a lane serving one is still reading the next request.
///
/// This is the break: the dispatcher used to serve a subscription by
/// looping on its items, which never polls the transport again. A room
/// that stays open — the normal case — took the lane with it.
#[tokio::test]
async fn a_subscription_does_not_block_the_lane() {
    let (mux, _stopped) = wired();

    // A room that will not finish on its own.
    let _quiet = mux.rpc_stream(Context::default(), Ask::Task(0), 8).await;

    let mut second = mux.rpc_stream(Context::default(), Ask::Task(3), 8).await;
    let got = drain(
        &mut second,
        "an open subscription must not stop the lane serving the next request",
    )
    .await;
    assert_eq!(got, vec![0, 1, 2]);
}

/// r[impl jetstream.subscription.cancel]
/// Cancellation arrives as a request on the lane, under a fresh tag,
/// naming its target. It must reach the producer, terminate the
/// subscription, and be acknowledged.
#[tokio::test]
async fn a_cancellation_frame_stops_the_producer() {
    let (mux, stopped) = wired();
    let mut quiet = mux.rpc_stream(Context::default(), Ask::Task(0), 8).await;
    let tag = quiet.tag;

    let ack = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        mux.rpc(
            Context::default(),
            Ask::Cancel(Tcancel {
                oldtag: tag,
                binding: 0,
            }),
        )
        .await,
    )
    .await
    .expect("a cancellation must be answered")
    .unwrap();
    assert_eq!(ack.msg, Say::Ack(Rcancel { oldtag: tag }));

    assert!(
        stopped.load(std::sync::atomic::Ordering::SeqCst),
        "cancellation must reach the work, not merely the delivery"
    );

    // r[impl jetstream.subscription.termination]
    // The subscription still ends with a terminator: without one the
    // caller's tag is held for the life of the lane.
    let last =
        tokio::time::timeout(std::time::Duration::from_secs(5), quiet.next())
            .await
            .expect("a cancelled subscription must terminate");
    assert_eq!(last.unwrap().unwrap().msg, Say::Done);
}

/// r[impl jetstream.subscription.surface.cancellation]
/// Dropping the stream is how a Rust caller cancels, so dropping it must
/// do everything cancelling does — including telling the producer.
#[tokio::test]
async fn dropping_the_subscription_stops_the_producer() {
    let (mux, stopped) = wired();
    let quiet = mux.rpc_stream(Context::default(), Ask::Task(0), 8).await;
    drop(quiet);

    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !stopped.load(std::sync::atomic::Ordering::SeqCst) {
        assert!(
            std::time::Instant::now() < deadline,
            "dropping the subscription must stop the producer"
        );
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}

/// r[impl jetstream.subscription.identity]
/// A cancelled subscription's tag comes free again, and only then. The
/// pool here is small enough that a single leaked tag stops the fourth
/// round — which is what a subscription ending with no terminator would
/// have done, since `r[jetstream.subscription.identity]` forbids
/// releasing the tag at drop.
#[tokio::test]
async fn a_cancelled_subscription_frees_its_tag() {
    let (mux, _stopped) = wired_with(2);
    for round in 0..8 {
        let mut quiet =
            mux.rpc_stream(Context::default(), Ask::Task(0), 8).await;
        let tag = quiet.tag;
        let ack = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            mux.rpc(
                Context::default(),
                Ask::Cancel(Tcancel {
                    oldtag: tag,
                    binding: 0,
                }),
            )
            .await,
        )
        .await
        .unwrap_or_else(|_| panic!("round {round} ran out of tags"))
        .unwrap();
        assert_eq!(ack.msg, Say::Ack(Rcancel { oldtag: tag }));
        // Drain to the terminator, which is what releases the tag.
        let mut got = drain(&mut quiet, "the terminator must arrive").await;
        got.clear();
    }
}

/// A subscription that ended has nothing to cancel. Dropping it must not
/// put a cancellation on the wire: the tag is already free, and a service
/// would have to answer a cancellation for a subscription it has
/// forgotten. The test plays the service itself, so it sees exactly which
/// requests arrive.
#[tokio::test]
async fn a_finished_subscription_sends_no_cancellation() {
    let (to_server, mut from_client) = fmpsc::unbounded();
    let (to_client, from_server) = fmpsc::unbounded();
    let mux = Mux::<Room>::new(
        4,
        Box::new(Duplex {
            tx: to_server,
            rx: from_server,
        }),
    );

    let mut items = mux.rpc_stream(Context::default(), Ask::Task(1), 8).await;
    let Some(Ok(Frame {
        tag,
        msg: Ask::Task(1),
    })) = from_client.next().await
    else {
        panic!("the subscription request must arrive first");
    };
    to_client
        .unbounded_send(Ok(Frame {
            tag,
            msg: Say::Item(0),
        }))
        .unwrap();
    to_client
        .unbounded_send(Ok(Frame {
            tag,
            msg: Say::Done,
        }))
        .unwrap();

    let got = drain(&mut items, "the subscription must complete").await;
    assert_eq!(got, vec![0]);
    drop(items);

    assert!(
        tokio::time::timeout(
            std::time::Duration::from_millis(200),
            from_client.next(),
        )
        .await
        .is_err(),
        "a subscription that already ended needs no cancelling"
    );
}

/// r[impl jetstream.subscription.dispatch.concurrent]
/// A producer that is always ready must not starve the inbound side.
///
/// The loop was `biased` with items first, reasoning that a ready
/// producer should not wait behind an inbound request. That is exactly
/// backwards: a producer which is ready on every poll — no timer, no
/// awaited source, which `unfold` over a counter already is — means the
/// item branch wins every time and the inbound branch is never selected.
/// The chatty subscription then prevents *its own* cancellation from
/// being read, which is the failure the whole dispatcher exists to
/// remove, reintroduced as an optimisation.
///
/// The assertion is on *promptness*, not eventual delivery: with the
/// biased loop the acknowledgement does arrive, after every one of the
/// items ahead of it, and a test that merely waited would pass.
#[tokio::test]
async fn a_chatty_producer_does_not_starve_its_own_cancellation() {
    let (to_service, service_rx) = fmpsc::unbounded();
    let (service_tx, mut from_service) = fmpsc::unbounded();

    tokio::spawn(async move {
        let mut room = Room {
            stopped: Default::default(),
        };
        let _ = crate::server::run(
            &mut room,
            ServiceSide {
                tx: service_tx,
                rx: service_rx,
            },
        )
        .await;
    });

    // Long enough to starve the inbound branch for the length of the
    // test, bounded so a regression fails rather than exhausts memory.
    to_service
        .unbounded_send(Ok(Frame {
            tag: 1,
            msg: Ask::Task(200_000),
        }))
        .unwrap();
    to_service
        .unbounded_send(Ok(Frame {
            tag: 2,
            msg: Ask::Cancel(crate::subscription::Tcancel::on_lane(1)),
        }))
        .unwrap();

    // How many frames pass before the cancellation is answered. Under
    // the biased loop this is every item the producer had queued.
    let mut before = 0usize;
    let acknowledged =
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while let Some(Ok(frame)) = from_service.next().await {
                if matches!(frame.msg, Say::Ack(_)) {
                    return true;
                }
                before += 1;
                if before > 5_000 {
                    return false;
                }
            }
            false
        })
        .await
        .expect("the dispatcher must answer a cancellation at all");
    assert!(
        acknowledged,
        "the cancellation went unanswered for {before} items: a producer \
         that is always ready starved the inbound branch"
    );
}

/// r[impl jetstream.subscription.cancel]
/// A cancellation draws from control capacity, not the ordinary pool.
///
/// The specification requires this and the first implementation ignored
/// it. The deadlock it prevents: a cancellation needs a *fresh* tag, the
/// subscription's own tag is not released until its terminator, and the
/// terminator cannot arrive until the cancellation is sent — so with the
/// pool saturated by live subscriptions, cancellation waits on the very
/// thing it would unblock.
///
/// Asserted on the tag's *region* rather than by reproducing the
/// deadlock, because a test that hangs on regression is worse than one
/// that fails: it takes the suite with it.
#[tokio::test]
async fn a_cancellation_uses_control_capacity() {
    let (to_service, mut service_rx) = fmpsc::unbounded();
    let (service_tx, from_service) = fmpsc::unbounded();
    let mux = Mux::<Room>::new(
        1,
        Box::new(Duplex {
            tx: to_service,
            rx: from_service,
        }),
    );
    drop(service_tx);

    // One subscription, then let it go.
    let items = mux.rpc_stream(Context::default(), Ask::Task(0), 8).await;
    let subscribe = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        service_rx.next(),
    )
    .await
    .expect("the subscription reaches the lane")
    .expect("a frame")
    .expect("a well-formed frame");
    drop(items);

    let cancel = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        service_rx.next(),
    )
    .await
    .expect("dropping a subscription must produce a cancellation")
    .expect("a frame")
    .expect("a well-formed frame");

    match cancel.msg {
        Ask::Cancel(t) => {
            assert_eq!(t.oldtag, subscribe.tag, "it names the subscription");
            assert_eq!(
                t.target_binding(),
                None,
                "on the subscription's own lane, so no binding is named"
            );
        }
        other => panic!("expected a cancellation, got {other:?}"),
    }

    // `TagPool::new(1)` gives ordinary `1..=1`, then a streaming region,
    // then control. The cancellation's own tag must come from the last of
    // those: above the subscription's, and far above the ordinary one.
    assert!(
        cancel.tag > subscribe.tag,
        "the cancellation's tag ({}) must come from a region above the \
         streaming one ({})",
        cancel.tag,
        subscribe.tag
    );
    assert!(
        cancel.tag > 1,
        "and never from the ordinary pool, which is what deadlocks"
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

    // Capacity one, never read: `Ask::Task(1)` sends a single item, which
    // fills the queue exactly, and then the terminator, which cannot fit.
    let lagging = mux.rpc_stream(Context::default(), Ask::Task(1), 1).await;
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
    let stuck = mux.rpc_stream(Context::default(), Ask::Task(0), 1).await;
    peer.send(Ok(Frame {
        tag: stuck.tag,
        msg: Say::Item(0),
    }))
    .await
    .unwrap();
    settle().await;

    // Behind it in tag order, and therefore behind it in the drain.
    let mut waiting = mux.rpc_stream(Context::default(), Ask::Task(0), 8).await;
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
    let _doomed = mux.rpc_stream(Context::default(), Ask::Task(0), 8).await;
    settle().await;

    let mut never = mux.rpc_stream(Context::default(), Ask::Task(0), 8).await;
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

    let mut done = mux.rpc_stream(Context::default(), Ask::Task(1), 8).await;
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

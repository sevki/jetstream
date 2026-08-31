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

use crate::{
    client::ClientTransport, context::Context, subscription::RDONE, Error,
    Frame, Framer, Mux, Protocol,
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
}

impl crate::server::Server for Room {
    // r[impl jetstream.subscription.surface.declared]
    fn is_streaming(message_type: u8) -> bool {
        message_type == TASK
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
        let Frame { tag, msg: Ask(n) } = frame;
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
                msg: Ask(4),
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
                msg: Ask(1_000_000),
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
            msg: Say::Item(frame.msg.0),
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
            msg: Ask(3),
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

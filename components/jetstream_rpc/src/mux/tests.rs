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

/// r[impl jetstream.subscription.compat.existing-clients]
/// Unary calls are untouched, and their tag is freed by the one response.
#[tokio::test]
async fn unary_calls_are_unaffected() {
    let (transport, _peer) = counting_transport();
    let mux = Mux::<Counting>::new(4, transport);
    let frame = mux.rpc(Context::default(), Ask(1)).await.await.unwrap();
    assert_eq!(frame.msg, Say::Item(0));
}

/// The panic this change removes. `demux` used to `unwrap` on a tag
/// nobody was waiting for, so any unsolicited or duplicate frame took the
/// whole client down — which is why a service could not push at all.
///
/// The check is that the client still *works* afterwards, not that the
/// test survives: `demux` runs in a spawned task, so a panic there kills
/// the task silently and a test that merely sleeps passes either way.
/// Verified by restoring the `unwrap` — this then hangs on the call
/// below, because the demultiplexer is dead and no response can arrive.
#[tokio::test]
async fn an_unknown_tag_does_not_panic_the_client() {
    let (transport, peer) = counting_transport();
    let mux = Mux::<Counting>::new(4, transport);

    // Nobody ever issued tag 9. This travels the real demultiplexer path,
    // which is where the old code took the task down.
    peer.unbounded_send(Ok(Frame {
        tag: 9,
        msg: Say::Item(1),
    }))
    .unwrap();

    // The demultiplexer must still be running: a real call afterwards
    // completes, and does so promptly.
    let items = mux.rpc_stream(Context::default(), Ask(2), 8).await;
    let got: Vec<u32> = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        items
            .filter_map(|f| async move {
                match f.ok()?.msg {
                    Say::Item(n) => Some(n),
                    Say::Done => None,
                }
            })
            .collect(),
    )
    .await
    .expect("the demultiplexer died with the unknown tag");
    assert_eq!(got, vec![0, 1]);
}

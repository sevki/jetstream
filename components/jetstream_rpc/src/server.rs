use std::{pin::pin, str::FromStr};

use futures::{Sink, Stream};
use jetstream_wireformat::WireFormat;
use tokio_util::{
    bytes::{self, Buf, BufMut},
    codec::{Decoder, Encoder},
};

use crate::{
    context::{Context, Contextual},
    Error, Frame, Framer, Protocol, Version,
};

pub struct ServerCodec<P: Protocol> {
    _phantom: std::marker::PhantomData<P>,
}

impl<P: Protocol> ServerCodec<P> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<P: Protocol> Default for ServerCodec<P> {
    fn default() -> Self {
        Self::new()
    }
}

pub trait ServiceTransport<P: Protocol>:
    Sink<Frame<P::Response>, Error = P::Error>
    + Stream<Item = Result<Frame<P::Request>, P::Error>>
    + Send
    + Sync
    + Unpin
{
    fn context(&self) -> Context;
}

impl<P: Protocol, T> ServiceTransport<P> for T
where
    T: Sink<Frame<P::Response>, Error = P::Error>
        + Stream<Item = Result<Frame<P::Request>, P::Error>>
        + Send
        + Sync
        + Unpin,
    T: Contextual,
{
    fn context(&self) -> Context {
        <Self as Contextual>::context(self)
    }
}

impl<P> Decoder for ServerCodec<P>
where
    P: Protocol,
{
    type Error = Error;
    type Item = Frame<P::Request>;

    fn decode(
        &mut self,
        src: &mut bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        // check to see if you have at least 4 bytes to figure out the size
        if src.len() < 4 {
            src.reserve(4);
            return Ok(None);
        }
        let Some(mut bytz) = src.get(..4) else {
            return Ok(None);
        };

        let byte_size: u32 = WireFormat::decode(&mut bytz)?;
        if src.len() < byte_size as usize {
            src.reserve(byte_size as usize);
            return Ok(None);
        }

        let frame = Frame::<P::Request>::decode(&mut src.reader())?;
        Ok(Some(frame))
    }
}

impl<P> Encoder<Frame<P::Response>> for ServerCodec<P>
where
    P: Protocol,
{
    type Error = Error;

    fn encode(
        &mut self,
        item: Frame<P::Response>,
        dst: &mut bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        item.encode(&mut dst.writer())?;
        Ok(())
    }
}

#[trait_variant::make(Send + Sync + Sized)]
pub trait Server: Protocol + Send + Sync {
    /// Negotiate the protocol version to use.
    fn version(client_version: Version) -> jetstream_error::Result<Version> {
        // By default, accept any version that matches the major version of the server's protocol version.
        let server_version =
            Version::from_str(Self::VERSION).unwrap_or_else(|_| {
                panic!(
                    "Invalid version format for JetStream protocol: {}",
                    Self::VERSION
                )
            });
        match (client_version, server_version) {
            (Version::V9P2000L, Version::V9P2000L) => Ok(Version::V9P2000L),
            (Version::V9P2000, Version::V9P2000) => Ok(Version::V9P2000),
            (
                Version::JetStream {
                    name: client_name,
                    version: client_version,
                },
                Version::JetStream {
                    name: server_name,
                    version: server_version,
                },
            ) => {
                // compare versions of client and server and send the lowest version
                if client_name != server_name {
                    return Err(Error::new(format!(
                        "Incompatible protocol names: client={}, server={}",
                        client_name, server_name
                    )));
                }
                Ok(Version::JetStream {
                    name: server_name,
                    version: client_version.min(server_version),
                })
            }
            _ => Err(Error::new("Incompatible protocols".to_string())),
        }
    }
    /// The main RPC method that handles incoming requests and produces responses.
    async fn rpc(
        &mut self,
        context: Context,
        frame: Frame<Self::Request>,
    ) -> Result<Frame<Self::Response>, Self::Error>;

    /// r[impl jetstream.subscription.surface.declared]
    /// Whether a request of this message type opens a subscription. The
    /// declaration is the protocol's, not the call site's, so the
    /// dispatcher can route before it moves the frame.
    ///
    /// Defaults to "no streaming methods", which is every protocol
    /// written before this existed.
    fn is_streaming(_message_type: u8) -> bool {
        false
    }

    /// r[impl jetstream.subscription.dispatch.declared]
    /// r[impl jetstream.subscription.cancel]
    /// Whether this request is a cancellation, and which subscription it
    /// names. Cancellation travels as an ordinary request under a fresh
    /// tag, so the dispatcher has to be told how to recognise one — the
    /// message id is global, but decoding the payload is the protocol's.
    ///
    /// Defaulted to "this protocol has no cancellation", which is
    /// correct for every protocol without subscriptions.
    fn cancel_target(_frame: &Frame<Self::Request>) -> Option<u16> {
        None
    }

    /// r[impl jetstream.subscription.dispatch.declared]
    /// r[impl jetstream.subscription.cancel]
    /// The acknowledgement for a cancelled subscription, sent under the
    /// *cancellation's* tag once the subscription has stopped emitting.
    fn cancel_ack(_oldtag: u16) -> Option<Self::Response> {
        None
    }

    /// r[impl jetstream.subscription.dispatch.terminator]
    /// r[impl jetstream.subscription.termination]
    /// The terminator for a subscription that was cut short. A producer
    /// stopped by cancellation returns from its stream without saying
    /// anything, and a subscription that ends with no terminator leaves
    /// the caller's tag in flight for the life of the lane — see
    /// `r[jetstream.subscription.identity]` for why it cannot simply be
    /// released. The dispatcher supplies one, and the protocol says what
    /// value it has.
    ///
    /// r[impl jetstream.subscription.termination.discriminant]
    /// `method` is the message type of the request that opened the
    /// subscription. A protocol with one streaming method can ignore it;
    /// one with two cannot, because the terminator's payload names its
    /// method and only the request says which method this was.
    fn cancelled_terminator(_method: u8) -> Option<Self::Response> {
        None
    }

    /// r[impl jetstream.subscription.overview]
    /// Serve a subscription: one request, many responses.
    ///
    /// r[impl jetstream.subscription.cancel]
    /// `cancel` is how the producer learns the subscriber has gone.
    /// Releasing only the delivery obligation is not enough — an
    /// inference or a build must be able to stop, not merely stop being
    /// listened to — so this is a parameter rather than something the
    /// dispatcher keeps to itself.
    ///
    /// Failures travel as items, per `r[jetstream.subscription.termination]`,
    /// which is also why the end can carry a value.
    ///
    /// r[impl jetstream.subscription.compat.rpc-layer]
    /// Defaulted, so every existing `Server` implementation compiles
    /// untouched: the breakage this change carries is on the client,
    /// where `RpcCall` resolves to one frame.
    async fn rpc_stream(
        &mut self,
        context: Context,
        frame: Frame<Self::Request>,
        cancel: tokio_util::sync::CancellationToken,
    ) -> ResponseStream<Self>
    where
        // The served stream is boxed and owned, so what it carries must
        // outlive the call that made it. Every real protocol's messages
        // are owned values, so this costs nothing but has to be said —
        // the same bound `Datagrams` needs, for the same reason.
        Self::Response: 'static,
        Self::Error: 'static,
    {
        let _ = (context, frame, cancel);
        // A bare async block, not a normal body: `#[trait_variant::make]`
        // rewrites `async fn` into `fn -> impl Future`, so the body must
        // *be* the future. The two natural spellings fail with errors
        // that point elsewhere entirely. An *implementation* of this
        // method writes an ordinary body, since the macro rewrites the
        // trait and not its impls.
        async move { Box::pin(futures::stream::empty()) as ResponseStream<Self> }
    }
}

/// The items a served subscription yields.
pub type ResponseStream<P> = std::pin::Pin<
    Box<
        dyn futures::Stream<
                Item = Result<
                    Frame<<P as Protocol>::Response>,
                    <P as Protocol>::Error,
                >,
            > + Send,
    >,
>;

/// One served subscription, tagged, and — unlike the stream it wraps —
/// able to say that it *ended*.
///
/// r[impl jetstream.subscription.surface.composition]
/// The same erasure the surface rule is about bites the dispatcher:
/// `SelectAll` drops a finished stream silently, so a subscription that
/// stopped would leave its tag open and its cancellation unacknowledged.
/// The end is a value here for exactly the reason it is one for callers.
struct Served<P: Protocol> {
    tag: u16,
    inner: Option<ResponseStream<P>>,
}

enum Out<P: Protocol> {
    Item(u16, Result<Frame<P::Response>, P::Error>),
    Ended(u16),
}

impl<P: Protocol> Stream for Served<P> {
    type Item = Out<P>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll;
        let this = self.get_mut();
        let tag = this.tag;
        let Some(inner) = this.inner.as_mut() else {
            return Poll::Ready(None);
        };
        match inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(item)) => Poll::Ready(Some(Out::Item(tag, item))),
            Poll::Ready(None) => {
                this.inner = None;
                Poll::Ready(Some(Out::Ended(tag)))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// What the dispatcher remembers about a subscription it is serving.
struct Open {
    cancel: tokio_util::sync::CancellationToken,
    /// The message type of the request that opened it. The terminator
    /// the dispatcher may have to supply names its method, and only the
    /// request knows which method that was.
    method: u8,
    /// Set when the producer emitted a terminator of its own, so the
    /// dispatcher does not send a second one.
    terminated: bool,
    /// The tags of the cancellations waiting to be acknowledged. The
    /// acknowledgement is owed *after* the last item.
    ///
    /// r[impl jetstream.subscription.cancel]
    /// A list rather than one tag: nothing stops a caller — or the
    /// client's own drop path, which queues a cancellation of its own —
    /// from sending two before the producer notices the first. Keeping
    /// only the latest silently drops the earlier one, and every
    /// cancellation is owed an answer; the tag it was sent under stays
    /// in flight until it gets one, so the ones forgotten here are
    /// control tags leaked for the life of the lane.
    ack_to: Vec<u16>,
}

pub async fn run<T, P>(p: &mut P, transport: T) -> Result<(), P::Error>
where
    T: ServiceTransport<P>,
    P: Server,
    P::Response: 'static,
    P::Error: 'static,
{
    use futures::{SinkExt, StreamExt};

    // The context is the transport's, not the frame's: it describes the
    // peer at the other end of this lane, which does not change between
    // requests. Reading it once is what lets the transport be split.
    let context = transport.context();

    // r[impl jetstream.subscription.dispatch.concurrent]
    // r[impl jetstream.subscription.definition]
    // Split, because a subscription is one call among the lane's others.
    // Serving one by looping on its items never polls the transport
    // again, so an open subscription — the normal case, a room that
    // stays open — took the lane with it: no second request served, and
    // no cancellation could arrive, which made the cancellation rule
    // unimplementable rather than merely unimplemented.
    let (mut sink, mut source) = transport.split();
    let mut active: futures::stream::SelectAll<Served<P>> =
        futures::stream::SelectAll::new();
    let mut open: std::collections::BTreeMap<u16, Open> =
        std::collections::BTreeMap::new();
    let mut a = pin!(p);

    // The reason the loop stopped, so the cleanup below runs on every
    // exit rather than only the tidy one.
    let outcome: Result<(), P::Error> = 'dispatch: loop {
        // r[impl jetstream.subscription.dispatch.concurrent]
        // Fair selection, deliberately. This was `biased` with items
        // first, on the reasoning that a ready producer should not wait
        // behind an inbound request — which is exactly backwards. An
        // always-ready producer, and `repeat` is enough, makes the item
        // branch ready on every iteration, so the inbound branch is never
        // selected and the chatty subscription prevents *its own*
        // cancellation from ever being read. That is the failure this
        // whole dispatcher exists to remove, reintroduced by an
        // optimisation.
        tokio::select! {
            Some(out) = active.next(), if !active.is_empty() => match out {
                Out::Item(tag, item) => {
                    let frame = match item {
                        Ok(frame) => frame,
                        Err(e) => break Err(e),
                    };
                    if frame.msg.message_type() == crate::subscription::RDONE {
                        if let Some(o) = open.get_mut(&tag) {
                            o.terminated = true;
                        }
                    }
                    if let Err(e) = sink.send(frame).await {
                        break Err(e);
                    }
                }
                Out::Ended(tag) => {
                    if let Some(o) = open.remove(&tag) {
                        // r[impl jetstream.subscription.dispatch.terminator]
                        if !o.terminated {
                            if let Some(msg) = P::cancelled_terminator(o.method) {
                                if let Err(e) =
                                    sink.send(Frame { tag, msg }).await
                                {
                                    break Err(e);
                                }
                            }
                        }
                        // r[impl jetstream.subscription.cancel]
                        // After every item already emitted, which is what
                        // makes the tag safe to reuse.
                        for ack_tag in &o.ack_to {
                            if let Some(msg) = P::cancel_ack(tag) {
                                if let Err(e) = sink
                                    .send(Frame {
                                        tag: *ack_tag,
                                        msg,
                                    })
                                    .await
                                {
                                    break 'dispatch Err(e);
                                }
                            }
                        }
                    }
                }
            },

            inbound = source.next() => {
                let Some(Ok(frame)) = inbound else { break Ok(()) };

                // r[impl jetstream.subscription.cancel]
                if let Some(oldtag) = P::cancel_target(&frame) {
                    match open.get_mut(&oldtag) {
                        Some(o) => {
                            o.ack_to.push(frame.tag);
                            o.cancel.cancel();
                        }
                        // Nothing by that tag: already finished, or never
                        // existed. Acknowledge anyway — a cancellation
                        // racing a terminator is normal, and the caller
                        // is owed an answer either way.
                        None => {
                            if let Some(msg) = P::cancel_ack(oldtag) {
                                if let Err(e) = sink
                                    .send(Frame { tag: frame.tag, msg })
                                    .await
                                {
                                    break Err(e);
                                }
                            }
                        }
                    }
                    continue;
                }

                // r[impl jetstream.subscription.surface.declared]
                // Routed on the declared message type, before the frame
                // moves.
                if P::is_streaming(frame.msg.message_type()) {
                    let tag = frame.tag;
                    let method = frame.msg.message_type();
                    // r[impl jetstream.subscription.cancel]
                    // The token the producer watches.
                    let cancel = tokio_util::sync::CancellationToken::new();
                    let inner =
                        a.rpc_stream(context.clone(), frame, cancel.clone())
                            .await;
                    open.insert(
                        tag,
                        Open {
                            cancel,
                            method,
                            terminated: false,
                            ack_to: Vec::new(),
                        },
                    );
                    active.push(Served { tag, inner: Some(inner) });
                } else {
                    let response = match a.rpc(context.clone(), frame).await {
                        Ok(response) => response,
                        Err(e) => break Err(e),
                    };
                    if let Err(e) = sink.send(response).await {
                        break Err(e);
                    }
                }
            }
        }
    };

    // r[impl jetstream.subscription.cancel]
    // The lane is gone, so every producer on it has lost its subscriber.
    // Leaving the tokens uncancelled would leave the work running with
    // nowhere to deliver — the very thing cancellation exists to stop.
    //
    // This runs on *every* exit. It used to sit after a body full of
    // `?`, so any stream or sink error returned straight past it: the
    // tokens were merely dropped, and a producer holding a clone — a
    // detached task, which is the shape `producing` creates — never
    // learned its lane had gone.
    for (_, o) in open {
        o.cancel.cancel();
    }
    outcome
}

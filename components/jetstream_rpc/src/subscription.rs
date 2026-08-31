//! Wire pieces for subscriptions.
//!
//! r[impl jetstream.subscription.overview]
//! A subscription is a streaming response: one request, many responses
//! sharing its tag, terminated explicitly. This module carries the three
//! things that shape needs on the wire and nothing else — the terminator,
//! cancellation, and the endpoint identifier. The client and server
//! plumbing that uses them lands separately.

use jetstream_wireformat::{Data, JetStreamWireFormat, WireFormat};

/// r[impl jetstream.subscription.compat]
/// The terminator, and cancellation, take **global** message ids in the
/// space below `MESSAGE_ID_START`, exactly as `RJETSTREAMERROR` already
/// does. That is what keeps `102 + 2 * index` intact: a streaming method
/// costs no extra per-method id, so `r[jetstream.rpc.swift.message-ids]`
/// and `r[jetstream.rpc.ts.message-ids]` do not change and no protocol is
/// re-generated for its unary methods.
///
/// 5, 6 and 7 are taken by the error frames; 100 and 101 by version. 106
/// and 107 are `TERROR`/`RERROR`, which overlap the generated ids for
/// method index 2 — latent today, since nothing decodes an `ErrorFrame`
/// on a service lane, but a reason to allocate here rather than there.
pub const RDONE: u8 = 8;

/// r[impl jetstream.subscription.cancel]
pub const TCANCEL: u8 = 9;
/// r[impl jetstream.subscription.cancel]
pub const RCANCEL: u8 = TCANCEL + 1;

/// r[impl jetstream.subscription.cancel]
/// Cancellation bears a **fresh** tag and names its target in the
/// payload, which is the shape 9P's `Tflush` uses and the one this
/// specification adopted after an earlier draft got the mechanics
/// backwards. Sending it under the subscription's own tag would put two
/// calls under one correlation key while that tag is still in flight.
#[derive(Debug, Clone, PartialEq, Eq, JetStreamWireFormat)]
pub struct Tcancel {
    /// The tag of the subscription being cancelled.
    pub oldtag: u16,
    /// r[impl jetstream.subscription.identity]
    /// Zero when the cancellation travels on the subscription's own lane,
    /// where `oldtag` is unambiguous. Otherwise the binding identifier,
    /// because tags are allocated per lane and a concurrent subscription
    /// elsewhere on the session may hold the same number.
    ///
    /// Zero is **reserved** and never a real binding, which is what makes
    /// the sentinel sound. Prefer [`Tcancel::on_lane`] and
    /// [`Tcancel::off_lane`] over writing this field: the specification
    /// used to recommend a counter starting at zero, so the first
    /// subscription of a session got a binding indistinguishable from
    /// "no binding" — and a receiver then resolved `oldtag` against the
    /// wrong lane.
    pub binding: u64,
}

impl Tcancel {
    /// Cancel a subscription on the lane this cancellation travels.
    /// `oldtag` is unambiguous there, so no binding is named.
    pub fn on_lane(oldtag: u16) -> Self {
        Tcancel { oldtag, binding: 0 }
    }

    /// Cancel a subscription living on some *other* lane of the session,
    /// naming it by its binding identifier.
    ///
    /// # Panics
    ///
    /// If `binding` is zero, which is reserved for the on-lane case.
    /// A caller reaching this has allocated bindings from a counter
    /// starting at zero — the allocation `r[jetstream.subscription.identity]`
    /// now forbids for exactly this reason.
    pub fn off_lane(oldtag: u16, binding: u64) -> Self {
        assert_ne!(
            binding, 0,
            "binding identifier zero is reserved for the on-lane case; \
             allocate bindings from one"
        );
        Tcancel { oldtag, binding }
    }

    /// The binding this names, or `None` when it is on the subscription's
    /// own lane.
    pub fn target_binding(&self) -> Option<u64> {
        (self.binding != 0).then_some(self.binding)
    }
}

/// r[impl jetstream.subscription.cancel]
/// The acknowledgement, which `r[jetstream.subscription.cancel]` requires
/// to arrive on the subscription's own lane after every item already
/// emitted there — that ordering is what makes the tag safe to reuse.
#[derive(Debug, Clone, PartialEq, Eq, JetStreamWireFormat)]
pub struct Rcancel {
    /// The tag that has now stopped emitting.
    pub oldtag: u16,
}

/// r[impl jetstream.lane.addressing]
/// The endpoint a subscription addresses within a peer — the room, the
/// object, the cell. An opaque byte string: codegen emits the client that
/// carries it and cannot know an application's naming, so any structure
/// is the application's to impose and no implementation's to interpret.
#[derive(Debug, Clone, PartialEq, Eq, JetStreamWireFormat)]
pub struct Endpoint(pub Data);

impl Endpoint {
    /// The endpoint naming nothing in particular: a peer that hosts one
    /// thing, which is every protocol written before this existed.
    pub fn root() -> Self {
        Endpoint(Data(Vec::new()))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0 .0
    }
}

impl From<&str> for Endpoint {
    fn from(name: &str) -> Self {
        Endpoint(Data(name.as_bytes().to_vec()))
    }
}

/// r[impl jetstream.subscription.termination]
/// A terminator that carries a typed value names the method it ends.
///
/// `RDONE` is one **global** message id, which is what keeps
/// `102 + 2 * index` intact for every other method — but a protocol with
/// two subscription methods then has two terminal types sharing one id,
/// and a decoder handed the type byte alone cannot tell which to decode.
/// The tag would disambiguate it, and a decoder does not see the tag.
///
/// So the payload names its method: the message id of the *request* that
/// opened the subscription, which is unique per method by construction.
/// One byte, and the two rules stop contradicting each other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Terminator<D> {
    /// The message id of the request that opened this subscription.
    pub method: u8,
    pub value: D,
}

impl<D: WireFormat> WireFormat for Terminator<D> {
    fn byte_size(&self) -> u32 {
        1 + self.value.byte_size()
    }

    fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        self.method.encode(w)?;
        self.value.encode(w)
    }

    fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        let method = u8::decode(r)?;
        Ok(Terminator {
            method,
            value: D::decode(r)?,
        })
    }
}

/// r[impl jetstream.subscription.surface.terminal-value]
/// r[impl jetstream.subscription.surface.composition]
/// What a subscription yields. The end is a **value in the sequence**,
/// not the absence of one.
///
/// Both rules need this and neither is satisfiable without it: a `Stream`
/// ends with `None`, which has nowhere to put a result, and `select_all`
/// drops a finished stream silently, so a merged subscription would
/// deliver every item and no terminator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item<T, D> {
    /// One item of the sequence.
    Next(T),
    /// The end, and what it ended with.
    Done(D),
}

impl<T, D> Item<T, D> {
    pub fn is_done(&self) -> bool {
        matches!(self, Item::Done(_))
    }

    pub fn into_next(self) -> Option<T> {
        match self {
            Item::Next(t) => Some(t),
            Item::Done(_) => None,
        }
    }

    pub fn into_done(self) -> Option<D> {
        match self {
            Item::Next(_) => None,
            Item::Done(d) => Some(d),
        }
    }
}

/// r[impl jetstream.subscription.surface]
/// The caller's end of a subscription, in the types the caller declared.
///
/// A `Stream` of `Item<T, D>`, so consuming one is consuming a stream —
/// and dropping one cancels it, because dropping the `RpcStream` inside
/// does. Nothing in the type says whether the subscription got a lane of
/// its own or a tag on a shared one, per
/// `r[jetstream.subscription.realisation.opaque]`.
pub struct Subscription<T, D> {
    source: Source<T, D>,
}

/// Where a subscription's items come from.
///
/// r[impl jetstream.subscription.surface.producer]
/// A subscription is one type on both sides of the call, because one
/// trait describes both — and the two sides need different things. A
/// caller needs a sequence. A producer needs cancellation, which the
/// trait's signature has nowhere to put: `fn events(&self, ctx, from)
/// -> Subscription<Event, Closed>` names no token, and adding one would
/// put a parameter in the caller's signature that means nothing to a
/// caller.
///
/// So a producer's subscription is a function *awaiting* its token, and
/// the dispatcher supplies it when it serves the call. Polled without
/// one — in a test, or in process — it gets a token nothing cancels,
/// which is the truth of that situation.
enum Source<T, D> {
    /// Opening: the request is not on the wire yet.
    ///
    /// r[impl jetstream.subscription.surface.rust]
    /// A subscription opens on first poll, which is what lets the
    /// method be the plain `fn` the surface calls for. It also means
    /// "subscribe, then act, then read" is a race — nothing reached the
    /// service. [`Subscription::establish`] is the way to say "now",
    /// and it exists because writing the usage kept tripping over its
    /// absence.
    Opening(
        std::pin::Pin<
            Box<dyn std::future::Future<Output = ItemStream<T, D>> + Send>,
        >,
    ),
    Open(ItemStream<T, D>),
    Awaiting(Box<dyn FnOnce(CancellationToken) -> ItemStream<T, D> + Send>),
    /// Momentarily empty while the one above is being run. A variant
    /// rather than an `Option` so replacing it needs no bound on `T`.
    Taken,
}

/// The boxed sequence a `Subscription` reads: items, then the end.
pub type ItemStream<T, D> = std::pin::Pin<
    Box<dyn futures::Stream<Item = jetstream_error::Result<Item<T, D>>> + Send>,
>;

impl<T, D> Subscription<T, D>
where
    T: Send + 'static,
    D: Send + 'static,
{
    /// Build one from the frames of a streaming call. `decode` is the
    /// generated protocol's: only it knows which response variant is an
    /// item and which is the terminator.
    /// `decode` returns `None` for a frame that ends the subscription
    /// **without** an item — the terminator of a subscription cut
    /// short, which frees the tag and carries no result, because a
    /// producer stopped by cancellation never produced one. Fabricating
    /// a default there would report a result that did not happen.
    pub fn from_frames<P, F>(stream: crate::RpcStream<P>, mut decode: F) -> Self
    where
        P: crate::Protocol + 'static,
        P::Response: 'static,
        F: FnMut(
                crate::Frame<P::Response>,
            ) -> jetstream_error::Result<Option<Item<T, D>>>
            + Send
            + 'static,
    {
        use futures::StreamExt;
        Subscription {
            source: Source::Open(Box::pin(
                stream
                    .map(move |frame| decode(frame?))
                    .take_while(|decoded| {
                        futures::future::ready(!matches!(decoded, Ok(None)))
                    })
                    .filter_map(|decoded| {
                        futures::future::ready(match decoded {
                            Ok(Some(item)) => Some(Ok(item)),
                            Ok(None) => None,
                            Err(e) => Some(Err(e)),
                        })
                    }),
            )),
        }
    }

    /// Build one that opens when it is first polled.
    ///
    /// r[impl jetstream.subscription.surface.rust]
    /// Opening a subscription needs a tag and a request on the wire,
    /// both of which are asynchronous — and the surface the
    /// specification shows is not: `fn events(&self, ..) ->
    /// Subscription<Event, Closed>`. Deferring the open to the first
    /// poll is what reconciles them, and it is what a stream does
    /// anyway: nothing happens until something reads.
    pub fn opening<F>(open: F) -> Self
    where
        F: std::future::Future<Output = Self> + Send + 'static,
    {
        Subscription {
            source: Source::Opening(Box::pin(async move {
                match open.await.source {
                    Source::Open(items) => items,
                    // An opener that hands back a producer's
                    // subscription has nothing to produce into; it gets
                    // a token nothing cancels, as it would on poll.
                    Source::Awaiting(produce) => {
                        produce(CancellationToken::new())
                    }
                    Source::Opening(inner) => inner.await,
                    Source::Taken => {
                        unreachable!("only set while being replaced")
                    }
                }
            })),
        }
    }

    /// Put the request on the wire now, rather than at the first read.
    ///
    /// r[impl jetstream.subscription.surface.rust]
    /// Opening lazily is what keeps the method signature a plain `fn`,
    /// and it makes "subscribe, then act, then read" a race: the act
    /// happens before the subscription exists. Awaiting this first
    /// removes that race **on one lane**, where
    /// `r[jetstream.lane.delivery-order]` then orders the request ahead
    /// of whatever follows it.
    ///
    /// It does **not** order a subscription against a call on a
    /// *different* lane — `r[jetstream.lane.no-cross-lane-order]` — and
    /// nothing can. A subscription that must not miss anything says
    /// where to start, and its producer replays from there; that is
    /// what the cursor in `r[jetstream.subscription.resume]` is for.
    pub async fn establish(&mut self) {
        if let Source::Opening(_) = self.source {
            let Source::Opening(open) =
                std::mem::replace(&mut self.source, Source::Taken)
            else {
                unreachable!("checked immediately above")
            };
            self.source = Source::Open(open.await);
        }
    }

    /// r[impl jetstream.subscription.surface.producer]
    /// Build the producer's side: a subscription that is handed its
    /// cancellation when the dispatcher serves it.
    pub fn served<F, S>(produce: F) -> Self
    where
        F: FnOnce(CancellationToken) -> S + Send + 'static,
        S: futures::Stream<Item = jetstream_error::Result<Item<T, D>>>
            + Send
            + 'static,
    {
        Subscription {
            source: Source::Awaiting(Box::new(move |cancel| {
                Box::pin(produce(cancel))
            })),
        }
    }

    /// r[impl jetstream.subscription.surface.producer]
    /// The ergonomic producer: a task holding a [`Producer`], which can
    /// send *and* learn that its subscriber has gone.
    ///
    /// `r[jetstream.subscription.detached.state]` is the reason this
    /// takes a closure rather than a sender: what the closure captures
    /// is the producer's whole state, and a handler that can only be
    /// written as "hold this sender" cannot be evicted and resumed.
    pub fn producing<F, Fut>(capacity: usize, produce: F) -> Self
    where
        F: FnOnce(Producer<T, D>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        Subscription::served(move |cancel| {
            let (producer, items) = channel::<T, D>(capacity, cancel);
            tokio::spawn(produce(producer));
            futures::StreamExt::map(items, Ok)
        })
    }

    /// Take the item sequence, giving the producer its cancellation.
    ///
    /// r[impl jetstream.subscription.cancel]
    /// This is the dispatcher's call, and it is where cancellation
    /// reaches the work. A subscription that is already a sequence — a
    /// caller's — ignores the token, which is right: nothing on that
    /// side produces.
    pub fn serve(self, cancel: CancellationToken) -> ItemStream<T, D> {
        match self.source {
            Source::Open(items) => items,
            Source::Awaiting(produce) => produce(cancel),
            // A caller's subscription served as a producer's: it opens
            // itself, and the token has nothing to cancel here.
            Source::Opening(open) => Box::pin(futures::StreamExt::flatten(
                futures::stream::once(open),
            )),
            Source::Taken => unreachable!("only set while being replaced"),
        }
    }

    /// Build one from any stream of already-decoded items — the shape a
    /// test, an in-process implementation, or a replay uses.
    pub fn from_items<S>(items: S) -> Self
    where
        S: futures::Stream<Item = jetstream_error::Result<Item<T, D>>>
            + Send
            + 'static,
    {
        Subscription {
            source: Source::Open(Box::pin(items)),
        }
    }

    /// r[impl jetstream.subscription.surface.composition]
    /// Tag this subscription so a merge can say which one spoke. The
    /// terminator survives the merge on its own — it is an `Item` — but
    /// *which* subscription ended does not, and that is the other half
    /// of what the rule requires.
    pub fn labelled<K>(self, key: K) -> Labelled<K, T, D>
    where
        K: Clone,
    {
        Labelled { key, inner: self }
    }
}

impl<T, D> futures::Stream for Subscription<T, D> {
    type Item = jetstream_error::Result<Item<T, D>>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        // A producer's subscription polled without a dispatcher gets a
        // token nothing cancels. Nothing here can cancel it, so saying
        // otherwise would be a lie.
        if let Source::Opening(open) = &mut self.source {
            match open.as_mut().poll(cx) {
                std::task::Poll::Ready(items) => {
                    self.source = Source::Open(items);
                }
                std::task::Poll::Pending => return std::task::Poll::Pending,
            }
        }
        if let Source::Awaiting(_) = self.source {
            let Source::Awaiting(produce) =
                std::mem::replace(&mut self.source, Source::Taken)
            else {
                unreachable!("checked immediately above")
            };
            self.source = Source::Open(produce(CancellationToken::new()));
        }
        match &mut self.source {
            Source::Open(items) => items.as_mut().poll_next(cx),
            _ => unreachable!("replaced above"),
        }
    }
}

/// A subscription that says which one it is. `Unpin`, so it goes
/// straight into `select_all`.
pub struct Labelled<K, T, D> {
    key: K,
    inner: Subscription<T, D>,
}

impl<K: Clone + Unpin, T, D> futures::Stream for Labelled<K, T, D> {
    /// The key accompanies the failure as well as the item.
    ///
    /// r[impl jetstream.subscription.surface.composition]
    /// This used to yield a bare `Err`, which throws away the one thing
    /// `Labelled` exists to keep. In a fan-in over several rooms, a
    /// transport failure, a decode failure or a producer failure ends
    /// one input and the caller could not tell which — so it could
    /// neither retry that room nor report it, which is the same loss the
    /// rule forbids for the *successful* end.
    type Item = (K, jetstream_error::Result<Item<T, D>>);

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll;
        let this = self.get_mut();
        let key = this.key.clone();
        match std::pin::Pin::new(&mut this.inner).poll_next(cx) {
            Poll::Ready(Some(result)) => Poll::Ready(Some((key, result))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// r[impl jetstream.subscription.surface.composition]
/// Merge many subscriptions without losing which one ended, or with
/// what. Fan-in — use case 2 — and a terminal value — use case 4 — are
/// each easy alone; this is the combination the document listed
/// separately and never checked.
pub fn merge<K, T, D, I>(
    subscriptions: I,
) -> futures::stream::SelectAll<Labelled<K, T, D>>
where
    K: Clone + Unpin,
    T: Send + 'static,
    D: Send + 'static,
    I: IntoIterator<Item = (K, Subscription<T, D>)>,
{
    futures::stream::select_all(
        subscriptions.into_iter().map(|(k, s)| s.labelled(k)),
    )
}

/// The producer stopped because its subscriber did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cancelled;

impl std::fmt::Display for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the subscription was cancelled")
    }
}

impl std::error::Error for Cancelled {}

/// The signal a producer watches. Re-exported so a producer need not
/// depend on `tokio-util` to write against this surface.
pub use tokio_util::sync::CancellationToken;

/// r[impl jetstream.subscription.surface.producer]
/// The producer's end. It can send, and it can **learn** — which is the
/// half a bare `Sender` cannot do, and the reason a producer loop written
/// against one compiles, runs, and keeps inferring long after the
/// subscriber has gone.
///
/// Cancellation is offered all three ways the rule allows: a signal to
/// await, a flag to test, and a send that fails once cancelled. A loop
/// that checks none of them still stops at its next `send`.
pub struct Producer<T, D> {
    tx: tokio::sync::mpsc::Sender<Item<T, D>>,
    cancel: tokio_util::sync::CancellationToken,
}

impl<T, D> Producer<T, D> {
    /// Emit one item. Fails once the subscriber has gone, so a loop that
    /// watches nothing else still terminates.
    pub async fn send(&self, item: T) -> Result<(), Cancelled> {
        tokio::select! {
            biased;
            _ = self.cancel.cancelled() => Err(Cancelled),
            sent = self.tx.send(Item::Next(item)) => {
                sent.map_err(|_| Cancelled)
            }
        }
    }

    /// r[impl jetstream.subscription.termination]
    /// End the subscription, carrying the result. Consuming `self` is
    /// the point: a producer that has finished cannot emit again.
    pub async fn finish(self, done: D) {
        let _ = self.tx.send(Item::Done(done)).await;
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    /// Resolves when the subscriber goes away — the shape a producer
    /// that is waiting on something else selects over.
    pub async fn cancelled(&self) {
        self.cancel.cancelled().await
    }

    /// The token itself, for a producer that hands cancellation onward
    /// to the work rather than watching it here.
    pub fn cancellation(&self) -> tokio_util::sync::CancellationToken {
        self.cancel.clone()
    }
}

/// The served side of a subscription: what the producer writes into.
pub struct Items<T, D> {
    rx: tokio::sync::mpsc::Receiver<Item<T, D>>,
}

impl<T, D> futures::Stream for Items<T, D> {
    type Item = Item<T, D>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

impl<T, D> Items<T, D>
where
    T: Send + 'static,
    D: Send + 'static,
{
    /// Turn decoded items into the frames a dispatcher serves. `encode`
    /// is the generated protocol's, for the same reason `decode` is.
    pub fn into_frames<P, F>(
        self,
        tag: u16,
        mut encode: F,
    ) -> crate::server::ResponseStream<P>
    where
        P: crate::Protocol + 'static,
        P::Response: 'static,
        P::Error: 'static,
        F: FnMut(Item<T, D>) -> P::Response + Send + 'static,
    {
        use futures::StreamExt;
        Box::pin(self.map(move |item| {
            Ok(crate::Frame {
                tag,
                msg: encode(item),
            })
        }))
    }
}

/// r[impl jetstream.subscription.surface.producer]
/// The producer/consumer pair for one subscription. `cancel` is the
/// dispatcher's token, so cancelling the subscription cancels the work.
pub fn channel<T, D>(
    capacity: usize,
    cancel: tokio_util::sync::CancellationToken,
) -> (Producer<T, D>, Items<T, D>) {
    let (tx, rx) = tokio::sync::mpsc::channel(capacity);
    (Producer { tx, cancel }, Items { rx })
}

#[cfg(test)]
mod tests;

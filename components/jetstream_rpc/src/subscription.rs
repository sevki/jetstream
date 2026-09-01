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
    items: ItemStream<T, D>,
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
    pub fn from_frames<P, F>(stream: crate::RpcStream<P>, mut decode: F) -> Self
    where
        P: crate::Protocol + 'static,
        P::Response: 'static,
        F: FnMut(
                crate::Frame<P::Response>,
            ) -> jetstream_error::Result<Item<T, D>>
            + Send
            + 'static,
    {
        use futures::StreamExt;
        // r[impl jetstream.subscription.surface.termination]
        // An ending is the last thing the subscription says. Mapping
        // each frame and leaving the stream running let a failure be
        // followed by more items, or even by a normal terminator — so a
        // subscriber could be told its subscription had failed and then
        // go on hearing from it. Through `merge` that is starker still:
        // an input reported as failed keeps speaking under the same key.
        Subscription {
            items: Box::pin(futures::stream::unfold(
                (stream, decode, false),
                move |(mut stream, mut decode, finished)| async move {
                    if finished {
                        return None;
                    }
                    let item = match stream.next().await? {
                        Ok(frame) => decode(frame),
                        Err(e) => Err(e),
                    };
                    let finished = matches!(item, Err(_) | Ok(Item::Done(_)));
                    Some((item, (stream, decode, finished)))
                },
            )),
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
            items: Box::pin(items),
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
        self.items.as_mut().poll_next(cx)
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
    tx: tokio::sync::mpsc::Sender<jetstream_error::Result<Item<T, D>>>,
    cancel: tokio_util::sync::CancellationToken,
}

impl<T, D> Producer<T, D> {
    /// Emit one item. Fails once the subscriber has gone, so a loop that
    /// watches nothing else still terminates.
    pub async fn send(&self, item: T) -> Result<(), Cancelled> {
        tokio::select! {
            biased;
            _ = self.cancel.cancelled() => Err(Cancelled),
            sent = self.tx.send(Ok(Item::Next(item))) => {
                sent.map_err(|_| Cancelled)
            }
        }
    }

    /// r[impl jetstream.subscription.termination]
    /// End the subscription, carrying the result. Consuming `self` is
    /// the point: a producer that has finished cannot emit again.
    pub async fn finish(self, done: D) {
        let _ = self.tx.send(Ok(Item::Done(done))).await;
    }

    /// r[impl jetstream.subscription.surface.termination]
    /// End the subscription because the work failed.
    ///
    /// Without this a producer had no way to say so. Returning early —
    /// the `?` in any ordinary producer loop — drops the `Producer`,
    /// which closes the channel, which the dispatcher reads as a
    /// subscription that simply stopped: it then supplies the terminator
    /// the caller is owed, and the subscriber receives a normal typed
    /// ending carrying a *fabricated* result. A failure reported as
    /// success is the one outcome the surface must never produce, so the
    /// channel carries failures and this is how one is sent.
    ///
    /// Dropping without calling either this or `finish` remains the
    /// cancelled case, which is what the dispatcher's synthetic
    /// terminator is for.
    pub async fn fail(self, error: jetstream_error::Error) {
        let _ = self.tx.send(Err(error)).await;
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
    rx: tokio::sync::mpsc::Receiver<jetstream_error::Result<Item<T, D>>>,
}

impl<T, D> futures::Stream for Items<T, D> {
    type Item = jetstream_error::Result<Item<T, D>>;

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
        F: FnMut(jetstream_error::Result<Item<T, D>>) -> P::Response
            + Send
            + 'static,
    {
        use futures::StreamExt;
        // `encode` sees the failure too, because only the protocol knows
        // how to say "this subscription failed" in its own response
        // type. Yielding `Err` here instead would make it a *transport*
        // error, which tears down the lane and every other call on it.
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

//! Presenting a plain byte stream as a session.
//!
//! r[impl jetstream.session.single-lane]
//! r[impl jetstream.session.conformance.single-stream]
//! TCP, TLS, unix sockets and stdio cannot open a second stream, but
//! they are still sessions: one lane, obtained once. The second open
//! fails with [`SessionError::LaneLimitReached`], which is distinct and
//! inspectable, rather than blocking or handing back the lane that is
//! already in use.

use std::{
    fmt,
    marker::PhantomData,
    pin::Pin,
    sync::{atomic::AtomicUsize, Arc},
    task::{Context as TaskContext, Poll},
};

use futures::{Sink, Stream};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Mutex,
};
use tokio_util::{codec::Framed, sync::CancellationToken};

use crate::{
    client::{ClientCodec, ClientTransport},
    context::{Context, Contextual},
    server::{ServerCodec, ServiceTransport},
    session::{
        capabilities::{Capabilities, IdentityKind, LaneSupport},
        error::SessionError,
        lifetime::{LaneGuard, LaneLifetime},
        Session,
    },
    Error, Frame, Protocol,
};

enum Never {}

impl<P: Protocol> fmt::Debug for NoClientLane<P> {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unreachable!("NoClientLane cannot be constructed")
    }
}

impl<P: Protocol> fmt::Debug for NoServiceLane<P> {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unreachable!("NoServiceLane cannot be constructed")
    }
}

impl<P: Protocol, C, S> fmt::Debug for SingleLaneSession<P, C, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SingleLaneSession")
            .field("lanes", &LaneSupport::One)
            .field("identity", &self.identity)
            .finish_non_exhaustive()
    }
}

/// The caller's end of a lane a session can never open.
///
/// Uninhabited: a session that names this as its
/// [`Session::ClientLane`] is one whose `open_lane` always fails, and
/// the type says so rather than a runtime convention saying it.
pub struct NoClientLane<P: Protocol> {
    _never: Never,
    _p: PhantomData<fn() -> P>,
}

impl<P: Protocol> Sink<Frame<P::Request>> for NoClientLane<P> {
    type Error = Error;

    fn poll_ready(
        self: Pin<&mut Self>,
        _cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        unreachable!("NoClientLane cannot be constructed")
    }

    fn start_send(
        self: Pin<&mut Self>,
        _item: Frame<P::Request>,
    ) -> Result<(), Self::Error> {
        unreachable!("NoClientLane cannot be constructed")
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        unreachable!("NoClientLane cannot be constructed")
    }

    fn poll_close(
        self: Pin<&mut Self>,
        _cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        unreachable!("NoClientLane cannot be constructed")
    }
}

impl<P: Protocol> Stream for NoClientLane<P> {
    type Item = Result<Frame<P::Response>, Error>;

    fn poll_next(
        self: Pin<&mut Self>,
        _cx: &mut TaskContext<'_>,
    ) -> Poll<Option<Self::Item>> {
        unreachable!("NoClientLane cannot be constructed")
    }
}

/// The callee's end of a lane a session can never accept.
///
/// Uninhabited, for the same reason as [`NoClientLane`]: a client on a
/// byte stream has nothing to accept.
pub struct NoServiceLane<P: Protocol> {
    _never: Never,
    _p: PhantomData<fn() -> P>,
}

impl<P: Protocol> Sink<Frame<P::Response>> for NoServiceLane<P> {
    type Error = Error;

    fn poll_ready(
        self: Pin<&mut Self>,
        _cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        unreachable!("NoServiceLane cannot be constructed")
    }

    fn start_send(
        self: Pin<&mut Self>,
        _item: Frame<P::Response>,
    ) -> Result<(), Self::Error> {
        unreachable!("NoServiceLane cannot be constructed")
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        unreachable!("NoServiceLane cannot be constructed")
    }

    fn poll_close(
        self: Pin<&mut Self>,
        _cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        unreachable!("NoServiceLane cannot be constructed")
    }
}

impl<P: Protocol> Stream for NoServiceLane<P> {
    type Item = Result<Frame<P::Request>, Error>;

    fn poll_next(
        self: Pin<&mut Self>,
        _cx: &mut TaskContext<'_>,
    ) -> Poll<Option<Self::Item>> {
        unreachable!("NoServiceLane cannot be constructed")
    }
}

impl<P: Protocol> Contextual for NoServiceLane<P> {
    fn context(&self) -> Context {
        unreachable!("NoServiceLane cannot be constructed")
    }
}

/// A session over a transport that carries exactly one lane.
///
/// Built from a lane that already exists — a framed TCP or unix stream —
/// and hands it out once.
pub struct SingleLaneSession<P: Protocol, C, S> {
    client: Mutex<Option<C>>,
    service: Mutex<Option<S>>,
    /// r[impl jetstream.session.lifetime]
    /// Cancelled by `close`. Every lane holds a child of it, so closing
    /// reaches a lane the caller already owns.
    cancel: CancellationToken,
    live: Arc<AtomicUsize>,
    opens: bool,
    accepts: bool,
    identity: IdentityKind,
    /// The identity the session established, where the lane underneath
    /// does not know it. `None` leaves the lane's own context alone.
    context: Option<Context>,
    _p: PhantomData<fn() -> P>,
}

impl<P: Protocol, C> SingleLaneSession<P, C, NoServiceLane<P>> {
    /// Present `lane` as the one lane this peer may open.
    pub fn client(lane: C) -> Self {
        Self {
            client: Mutex::new(Some(lane)),
            service: Mutex::new(None),
            cancel: CancellationToken::new(),
            live: Arc::new(AtomicUsize::new(0)),
            opens: true,
            accepts: false,
            identity: IdentityKind::None,
            context: None,
            _p: PhantomData,
        }
    }
}

impl<P: Protocol, S> SingleLaneSession<P, NoClientLane<P>, S> {
    /// Present `lane` as the one lane this peer may accept.
    pub fn service(lane: S) -> Self {
        Self {
            client: Mutex::new(None),
            service: Mutex::new(Some(lane)),
            cancel: CancellationToken::new(),
            live: Arc::new(AtomicUsize::new(0)),
            opens: false,
            accepts: true,
            identity: IdentityKind::None,
            context: None,
            _p: PhantomData,
        }
    }
}

/// A lane that reports an identity supplied by its builder.
///
/// r[impl jetstream.session.identity]
/// `Contextual` is implemented for a framed unix or TCP socket, which
/// know their peer. A TLS stream, stdio, or an in-memory duplex does
/// not: the identity either comes from a handshake the framed bytes
/// never see, or does not exist. Wrapping the lane lets those become
/// service lanes at all, with the caller stating what it knows.
pub struct ContextualLane<L> {
    lane: L,
    context: Context,
}

impl<L> ContextualLane<L> {
    /// Wrap `lane`, reporting `context` as its peer identity.
    pub fn new(lane: L, context: Context) -> Self {
        Self { lane, context }
    }

    /// The wrapped lane.
    pub fn get_ref(&self) -> &L {
        &self.lane
    }
}

impl<L, Item> Sink<Item> for ContextualLane<L>
where
    L: Sink<Item, Error = Error> + Unpin,
{
    type Error = Error;

    fn poll_ready(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.get_mut().lane).poll_ready(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        Pin::new(&mut self.get_mut().lane).start_send(item)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.get_mut().lane).poll_flush(cx)
    }

    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.get_mut().lane).poll_close(cx)
    }
}

impl<L, T> Stream for ContextualLane<L>
where
    L: Stream<Item = Result<T, Error>> + Unpin,
{
    type Item = Result<T, Error>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().lane).poll_next(cx)
    }
}

impl<L> Contextual for ContextualLane<L> {
    fn context(&self) -> Context {
        self.context.clone()
    }
}

/// A byte stream framed as this peer's one lane.
pub type ByteStreamLane<P, T> = Framed<T, ClientCodec<P>>;

/// A byte stream framed as the one lane this peer serves.
pub type ByteStreamServiceLane<P, T> = Framed<T, ServerCodec<P>>;

impl<P, T> SingleLaneSession<P, ByteStreamLane<P, T>, NoServiceLane<P>>
where
    P: Protocol,
    T: AsyncRead + AsyncWrite + Send + Sync + Unpin,
{
    /// r[impl jetstream.session.conformance.single-stream]
    /// Frame a byte stream — TCP, TLS, a unix socket, stdio — as the one
    /// lane this peer can open. Its capabilities are
    /// [`Capabilities::byte_stream`] unless the caller reports an
    /// identity the stream established, as a TLS stream would.
    pub fn client_io(io: T) -> Self {
        Self::client(Framed::new(io, ClientCodec::default()))
    }
}

impl<P, T> SingleLaneSession<P, NoClientLane<P>, ByteStreamServiceLane<P, T>>
where
    P: Protocol,
    T: AsyncRead + AsyncWrite + Send + Sync + Unpin,
    // Named here rather than left to the `Session` impl so that a stream
    // which cannot report its peer fails at this constructor, pointing
    // at `service_io_with_context`, instead of at a missing `Session`
    // impl further away.
    ByteStreamServiceLane<P, T>: Contextual,
{
    /// r[impl jetstream.session.conformance.single-stream]
    /// Frame an accepted byte stream as the one lane this peer serves.
    ///
    /// For a stream that knows its peer — a unix socket or TCP. Anything
    /// else wants [`Self::service_io_with_context`].
    pub fn service_io(io: T) -> Self {
        Self::service(Framed::new(io, ServerCodec::new()))
    }
}

impl<P, T>
    SingleLaneSession<
        P,
        NoClientLane<P>,
        ContextualLane<ByteStreamServiceLane<P, T>>,
    >
where
    P: Protocol,
    T: AsyncRead + AsyncWrite + Send + Sync + Unpin,
{
    /// r[impl jetstream.session.conformance.single-stream]
    /// Frame an accepted byte stream whose peer identity the caller
    /// supplies rather than the stream knowing it.
    ///
    /// r[impl jetstream.session.identity]
    /// This is the path for a TLS stream, whose peer is established by a
    /// handshake the framed bytes never see, and for stdio or an
    /// in-memory stream, which have no peer at all — pass
    /// [`Context::default`] for those.
    pub fn service_io_with_context(io: T, context: Context) -> Self {
        Self::service(ContextualLane::new(
            Framed::new(io, ServerCodec::new()),
            context.clone(),
        ))
        // The session reports it too, so a caller can inspect the peer
        // before accepting rather than only through the lane.
        .with_context(context)
    }
}

impl<P: Protocol, C, S> SingleLaneSession<P, C, S> {
    fn is_closed(&self) -> bool {
        self.cancel.is_cancelled()
    }

    /// r[impl jetstream.session.lifetime]
    /// A lane's share of this session's lifetime. Taking a child of an
    /// already-cancelled token yields an already-cancelled child, so a
    /// lane handed out concurrently with `close` is born terminated
    /// rather than escaping it.
    fn lifetime(&self) -> LaneLifetime {
        LaneLifetime::new(self.cancel.child_token(), self.live.clone())
    }

    /// How many lanes this session is currently keeping alive.
    pub fn live_lanes(&self) -> usize {
        self.live.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Report a peer identity model other than
    /// [`IdentityKind::None`] — a TLS stream authenticates its peer even
    /// though it carries one lane.
    pub fn with_identity(mut self, identity: IdentityKind) -> Self {
        self.identity = identity;
        self
    }

    /// Attach the peer identity the transport established.
    ///
    /// r[impl jetstream.session.identity]
    /// A lane accepted from this session reports this identity rather
    /// than whatever the framed stream underneath knows, so a TLS
    /// adapter's authenticated peer reaches the handler.
    pub fn with_context(mut self, context: Context) -> Self {
        self.context = Some(context);
        self
    }
}

/// r[impl jetstream.session.lifetime]
/// A lane must not outlive its session, so a session that goes away
/// without `close` — dropped during error unwinding, say — terminates
/// its lane just the same.
impl<P: Protocol, C, S> Drop for SingleLaneSession<P, C, S> {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

#[async_trait::async_trait]
impl<P, C, S> Session<P> for SingleLaneSession<P, C, S>
where
    P: Protocol<Error = Error>,
    C: ClientTransport<P>,
    // `Contextual` is what the blanket `ServiceTransport` impl is built
    // on; naming it here lets the guard forward the peer identity.
    S: ServiceTransport<P> + Contextual,
{
    type ClientLane = LaneGuard<C>;
    type ServiceLane = LaneGuard<S>;

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            lanes: LaneSupport::One,
            datagrams: false,
            identity: self.identity,
            migration: false,
        }
    }

    fn context(&self) -> Context {
        self.context.clone().unwrap_or_default()
    }

    async fn open_lane(&self) -> Result<Self::ClientLane, SessionError> {
        if self.is_closed() {
            return Err(SessionError::Closed);
        }
        let lane = self.client.lock().await.take().ok_or(if self.opens {
            SessionError::LaneLimitReached
        } else {
            SessionError::OpenUnsupported
        })?;
        Ok(LaneGuard::new(lane, self.lifetime()))
    }

    async fn accept_lane(&self) -> Result<Self::ServiceLane, SessionError> {
        if self.is_closed() {
            return Err(SessionError::Closed);
        }
        let lane = self.service.lock().await.take().ok_or(if self.accepts {
            SessionError::LaneLimitReached
        } else {
            SessionError::AcceptUnsupported
        })?;
        // r[impl jetstream.session.identity]
        // The handler sees the session's identity where there is one.
        Ok(LaneGuard::with_context(
            lane,
            self.lifetime(),
            self.context.clone(),
        ))
    }

    /// r[impl jetstream.session.lifetime]
    /// Terminates the lane whether or not it has been handed out:
    /// cancelling reaches a lane the caller already owns, so a call in
    /// flight on it fails rather than continuing to run on a session
    /// that has closed. A lane handed out concurrently with this call is
    /// born cancelled rather than escaping it.
    async fn close(&self) {
        self.cancel.cancel();
        self.client.lock().await.take();
        self.service.lock().await.take();
    }
}

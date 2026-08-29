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
    task::{Context as TaskContext, Poll},
};

use futures::{Sink, Stream};
use tokio::sync::Mutex;

use crate::{
    client::ClientTransport,
    context::{Context, Contextual},
    server::ServiceTransport,
    session::{
        capabilities::{Capabilities, IdentityKind, LaneSupport},
        error::SessionError,
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
    type Error = P::Error;

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
    type Item = Result<Frame<P::Request>, P::Error>;

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
    opens: bool,
    accepts: bool,
    identity: IdentityKind,
    context: Context,
    _p: PhantomData<fn() -> P>,
}

impl<P: Protocol, C> SingleLaneSession<P, C, NoServiceLane<P>> {
    /// Present `lane` as the one lane this peer may open.
    pub fn client(lane: C) -> Self {
        Self {
            client: Mutex::new(Some(lane)),
            service: Mutex::new(None),
            opens: true,
            accepts: false,
            identity: IdentityKind::None,
            context: Context::default(),
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
            opens: false,
            accepts: true,
            identity: IdentityKind::None,
            context: Context::default(),
            _p: PhantomData,
        }
    }
}

impl<P: Protocol, C, S> SingleLaneSession<P, C, S> {
    /// Report a peer identity model other than
    /// [`IdentityKind::None`] — a TLS stream authenticates its peer even
    /// though it carries one lane.
    pub fn with_identity(mut self, identity: IdentityKind) -> Self {
        self.identity = identity;
        self
    }

    /// Attach the peer identity the transport established.
    pub fn with_context(mut self, context: Context) -> Self {
        self.context = context;
        self
    }
}

#[async_trait::async_trait]
impl<P, C, S> Session<P> for SingleLaneSession<P, C, S>
where
    P: Protocol,
    C: ClientTransport<P>,
    S: ServiceTransport<P>,
{
    type ClientLane = C;
    type ServiceLane = S;

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            lanes: LaneSupport::One,
            datagrams: false,
            identity: self.identity,
            migration: false,
        }
    }

    fn context(&self) -> Context {
        self.context.clone()
    }

    async fn open_lane(&self) -> Result<Self::ClientLane, SessionError> {
        self.client.lock().await.take().ok_or(if self.opens {
            SessionError::LaneLimitReached
        } else {
            SessionError::OpenUnsupported
        })
    }

    async fn accept_lane(&self) -> Result<Self::ServiceLane, SessionError> {
        self.service.lock().await.take().ok_or(if self.accepts {
            SessionError::LaneLimitReached
        } else {
            SessionError::AcceptUnsupported
        })
    }

    /// Drops the lane if it has not been handed out yet.
    ///
    /// A byte-stream session *is* its lane: once the caller holds it,
    /// closing the session is closing the lane, and the caller does that
    /// by dropping what it holds.
    async fn close(&self) {
        self.client.lock().await.take();
        self.service.lock().await.take();
    }
}

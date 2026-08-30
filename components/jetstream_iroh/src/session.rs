//! The iroh connection as a JetStream session.
//!
//! r[impl jetstream.session.conformance.iroh]
//! An iroh `Connection` carries as many streams as the peers care to
//! open, and its peer identity is a public key that is also the address.
//! Opening one bidirectional stream at connect time and holding on to
//! nothing else throws both away. [`IrohSession`] keeps the connection
//! and hands out a lane per `open_bi`, so a caller decides how many
//! lanes it wants and when.

use std::{
    marker::PhantomData,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context as TaskContext, Poll},
};

use futures::{Sink, SinkExt, Stream, StreamExt};
use iroh::{
    endpoint::{
        Connection, ConnectionError, RecvStream, SendDatagramError, SendStream,
        VarInt,
    },
    Endpoint,
};
use jetstream_rpc::{
    context::{Context, Contextual, NodeId},
    server::ServerCodec,
    session::{Capabilities, Datagrams, Session, SessionError},
    Error, Frame, IntoError, Protocol,
};
use tokio_util::{
    bytes::Bytes,
    codec::{FramedRead, FramedWrite},
};

use crate::IrohTransport;

/// The code an iroh connection is closed with when its session closes.
const SESSION_CLOSED: u32 = 0;

/// r[impl jetstream.session.lifetime]
/// A connection error that means the session ended deliberately, rather
/// than failed. Used everywhere a connection error crosses into
/// `SessionError` — lanes and datagrams alike.
///
/// `SessionError::Closed` is what a caller branches on to stop retrying,
/// so mapping every `ConnectionError` to `Transport` told it that our
/// own `close` — and a peer's — was a transport fault. `LocallyClosed`
/// is this end calling `close`; `ApplicationClosed` is the far end doing
/// the same, since `close` sends an application code. Everything else —
/// a reset, an idle timeout, a protocol violation — really is a
/// failure and keeps its own error.
fn connection_error(err: ConnectionError) -> SessionError {
    match err {
        ConnectionError::LocallyClosed
        | ConnectionError::ApplicationClosed(_) => SessionError::Closed,
        other => SessionError::Transport(other.into_error()),
    }
}

/// What every handle on one session shares.
///
/// r[impl jetstream.session.lifetime]
/// A lane must not outlive its session, and every lane carries its own
/// clone of the connection — deliberately, so iroh does not tear the
/// connection down under a live stream. Those clones would keep the
/// association up after the session that made it had gone, so the
/// connection lives here and the last handle away closes it.
#[derive(Debug)]
struct SessionInner {
    connection: Connection,
    /// r[impl jetstream.session.capabilities]
    /// Cleared by [`IrohSession::without_datagrams`]. Shared rather than
    /// per-handle: whether this *connection* carries datagrams is a
    /// property of the connection, so a clone taken before the override
    /// must not go on claiming they work.
    datagrams: AtomicBool,
    // Held for as long as the session is, for the same reason
    // `IrohTransport` holds them: iroh tears the connection down
    // ungracefully if the `Endpoint` is dropped while streams opened
    // from it are still in use.
    endpoint: Option<Endpoint>,
}

impl Drop for SessionInner {
    /// r[impl jetstream.session.lifetime]
    /// A session that goes away during unwinding, rather than through
    /// `close`, ends its lanes just the same.
    fn drop(&mut self) {
        self.connection
            .close(VarInt::from_u32(SESSION_CLOSED), b"session dropped");
    }
}

/// A JetStream session over an iroh connection.
///
/// Cloning the session is cheap and every clone addresses the same
/// connection, so lanes can be opened from several tasks at once. The
/// connection closes when the last clone goes away.
pub struct IrohSession<P: Protocol> {
    inner: Arc<SessionInner>,
    _p: PhantomData<fn() -> P>,
}

// Written out rather than derived: a derive would require `P: Clone`
// and `P: Debug`, which a session neither holds nor needs — the
// protocol appears only as a `PhantomData` marker.
impl<P: Protocol> Clone for IrohSession<P> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _p: PhantomData,
        }
    }
}

impl<P: Protocol> std::fmt::Debug for IrohSession<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IrohSession")
            .field("connection", &self.inner.connection)
            .finish_non_exhaustive()
    }
}

impl<P: Protocol> IrohSession<P> {
    /// Wrap a connection whose endpoint the caller keeps alive.
    ///
    /// The session takes ownership of the connection's lifetime: when
    /// the last handle is dropped the connection closes, even if the
    /// caller holds a clone of it.
    pub fn new(connection: Connection) -> Self {
        Self {
            inner: Arc::new(SessionInner {
                connection,
                endpoint: None,
                datagrams: AtomicBool::new(true),
            }),
            _p: PhantomData,
        }
    }

    /// Wrap a connection and keep `endpoint` alive alongside it.
    pub fn new_owned(connection: Connection, endpoint: Endpoint) -> Self {
        Self {
            inner: Arc::new(SessionInner {
                connection,
                endpoint: Some(endpoint),
                datagrams: AtomicBool::new(true),
            }),
            _p: PhantomData,
        }
    }

    /// The underlying connection.
    pub fn connection(&self) -> &Connection {
        &self.inner.connection
    }

    /// Report this session as carrying no datagrams.
    ///
    /// r[impl jetstream.session.capabilities]
    /// [`Session::capabilities`] derives datagram support from
    /// `max_datagram_size`, which underneath reads the *peer's*
    /// advertised limit and the path MTU. It is silent about this
    /// endpoint's own configuration: an endpoint built with datagrams
    /// switched off still sees a size, while every send fails and
    /// nothing can arrive. There is no way to read that back from the
    /// connection, so the caller that turned them off says so here.
    ///
    /// The override is shared by every handle on this connection,
    /// including clones taken before the call: a clone that went on
    /// claiming datagrams work would recreate exactly the mismatch this
    /// is here to prevent.
    pub fn without_datagrams(self) -> Self {
        self.inner.datagrams.store(false, Ordering::SeqCst);
        self
    }
}

#[async_trait::async_trait]
impl<P> Session<P> for IrohSession<P>
where
    P: Protocol<Error = Error> + 'static,
{
    type ClientLane = IrohTransport<P>;
    type ServiceLane = IrohServiceLane<P>;

    /// r[impl jetstream.session.capabilities]
    /// r[impl jetstream.session.capabilities.degradation]
    /// The conformance row says iroh *can* carry datagrams. This
    /// connection may not: a peer that never advertised DATAGRAM
    /// support, or a transport configuration with them switched off,
    /// leaves `max_datagram_size` empty and every send failing. A
    /// capability is what this session has, not what its transport is
    /// capable of in general — reporting the row unconditionally would
    /// let `require(Capability::Datagrams)` succeed on a session that
    /// cannot carry one.
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            datagrams: <Self as Datagrams<P>>::max_datagram_size(self)
                .is_some(),
            ..Capabilities::iroh()
        }
    }

    /// r[impl jetstream.session.identity]
    /// iroh establishes the peer's public key during the handshake, so
    /// the identity is available without the peer asserting anything.
    fn context(&self) -> Context {
        Context::from(NodeId::from(self.inner.connection.remote_id()))
    }

    /// r[impl jetstream.lane.independence]
    /// Every lane is its own QUIC stream, so one lane's stalled frame
    /// does not delay another's.
    async fn open_lane(&self) -> Result<Self::ClientLane, SessionError> {
        let streams = self
            .inner
            .connection
            .open_bi()
            .await
            .map_err(connection_error)?;

        Ok(match &self.inner.endpoint {
            Some(endpoint) => IrohTransport::new_owned(
                streams,
                self.inner.connection.clone(),
                endpoint.clone(),
            ),
            None => IrohTransport::from(streams),
        })
    }

    /// r[impl jetstream.session.symmetric]
    /// Either peer may open a lane on an iroh connection, so a client
    /// that accepts is not doing anything unusual.
    async fn accept_lane(&self) -> Result<Self::ServiceLane, SessionError> {
        let (send_stream, recv_stream) = self
            .inner
            .connection
            .accept_bi()
            .await
            .map_err(connection_error)?;

        Ok(IrohServiceLane {
            send_stream: FramedWrite::new(send_stream, ServerCodec::new()),
            recv_stream: FramedRead::new(recv_stream, ServerCodec::new()),
            context: <Self as Session<P>>::context(self),
        })
    }

    /// r[impl jetstream.session.lifetime]
    /// Closing the connection resets every stream opened on it, so lanes
    /// terminate with the session rather than outliving it, and calls in
    /// flight on them fail rather than hang.
    async fn close(&self) {
        self.inner
            .connection
            .close(VarInt::from_u32(SESSION_CLOSED), b"session closed");
    }
}

/// r[impl jetstream.session.datagrams]
/// iroh carries QUIC datagrams, which belong to the connection and not
/// to any stream on it. Only the raw channel is bound here; framing is
/// the model's, so both peers agree on it.
#[async_trait::async_trait]
impl<P> Datagrams<P> for IrohSession<P>
where
    P: Protocol<Error = Error> + 'static,
    P::Request: 'static,
    P::Response: 'static,
{
    fn max_datagram_size(&self) -> Option<u32> {
        if !self.inner.datagrams.load(Ordering::SeqCst) {
            return None;
        }
        self.inner
            .connection
            .max_datagram_size()
            .map(|size| size.min(u32::MAX as usize) as u32)
    }

    async fn send_datagram_bytes(
        &self,
        bytes: Bytes,
    ) -> Result<(), SessionError> {
        self.inner.connection.send_datagram(bytes).map_err(|err| {
            // r[impl jetstream.session.lifetime]
            // A send that failed because the session ended says so, the
            // same as a lane open does. The other variants — no peer
            // support, disabled locally, too large — are the transport
            // reporting on the datagram itself.
            match err {
                SendDatagramError::ConnectionLost(err) => connection_error(err),
                other => SessionError::Transport(other.into_error()),
            }
        })
    }

    async fn recv_datagram_bytes(&self) -> Result<Bytes, SessionError> {
        self.inner
            .connection
            .read_datagram()
            .await
            .map_err(connection_error)
    }
}

/// The callee's end of a lane the peer opened.
///
/// r[impl jetstream.lane.definition]
/// One iroh bidirectional stream, framed, satisfying
/// `ServiceTransport<P>`.
pub struct IrohServiceLane<P: Protocol> {
    send_stream: FramedWrite<SendStream, ServerCodec<P>>,
    recv_stream: FramedRead<RecvStream, ServerCodec<P>>,
    context: Context,
}

impl<P: Protocol> IrohServiceLane<P> {
    /// Build a lane from a stream pair the caller accepted itself.
    pub fn new(
        send_stream: SendStream,
        recv_stream: RecvStream,
        context: Context,
    ) -> Self {
        Self {
            send_stream: FramedWrite::new(send_stream, ServerCodec::new()),
            recv_stream: FramedRead::new(recv_stream, ServerCodec::new()),
            context,
        }
    }
}

impl<P: Protocol> Sink<Frame<P::Response>> for IrohServiceLane<P> {
    type Error = Error;

    fn poll_ready(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.get_mut().send_stream.poll_ready_unpin(cx)
    }

    fn start_send(
        self: Pin<&mut Self>,
        item: Frame<P::Response>,
    ) -> Result<(), Self::Error> {
        self.get_mut().send_stream.start_send_unpin(item)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.get_mut().send_stream.poll_flush_unpin(cx)
    }

    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.get_mut().send_stream.poll_close_unpin(cx)
    }
}

impl<P: Protocol> Stream for IrohServiceLane<P> {
    type Item = Result<Frame<P::Request>, Error>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.get_mut().recv_stream.poll_next_unpin(cx)
    }
}

impl<P: Protocol> Contextual for IrohServiceLane<P> {
    fn context(&self) -> Context {
        self.context.clone()
    }
}

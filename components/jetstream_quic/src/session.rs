//! The QUIC connection as a JetStream session.
//!
//! r[impl jetstream.session.conformance]
//! A QUIC connection carries many streams, authenticates its peer with a
//! TLS certificate, and survives a change of network path. Each lane is
//! one bidirectional stream on it.

use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context as TaskContext, Poll},
};

use bytes::Bytes;
use futures::{Sink, SinkExt, Stream, StreamExt};
use jetstream_rpc::{
    context::{Context, Contextual, Peer, RemoteAddr, TlsPeer},
    server::ServerCodec,
    session::{
        check_datagram_size, Capabilities, Datagrams, Session, SessionError,
    },
    Error, Frame, IntoError, Protocol,
};
use jetstream_wireformat::WireFormat;
use quinn::{Connection, RecvStream, SendStream, VarInt};
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::warn;

use crate::client::QuicTransport;

/// The code a QUIC connection is closed with when its session closes.
const SESSION_CLOSED: u32 = 0;

/// r[impl jetstream.session.identity]
/// The peer's TLS certificate chain, parsed, or `None` when the
/// connection carries no client certificate.
///
/// For the default rustls session the dynamic identity downcasts to a
/// `Vec<CertificateDer>`.
pub fn peer_from_connection(connection: &Connection) -> Option<Peer> {
    let identity = connection.peer_identity()?;
    let certs = identity
        .downcast::<Vec<rustls::pki_types::CertificateDer>>()
        .ok()?;

    match TlsPeer::from_der_chain(
        &certs.iter().map(|cert| cert.as_ref()).collect::<Vec<_>>(),
    ) {
        Ok(tls_peer) => Some(Peer::Tls(tls_peer)),
        Err(err) => {
            warn!("failed to parse peer certificates: {}", err);
            None
        }
    }
}

/// A JetStream session over a QUIC connection.
///
/// Cloning is cheap and every clone addresses the same connection, so
/// lanes can be opened from several tasks at once.
#[derive(Debug, Clone)]
pub struct QuicSession<P: Protocol> {
    connection: Connection,
    _p: PhantomData<fn() -> P>,
}

impl<P: Protocol> QuicSession<P> {
    /// Wrap an established connection.
    pub fn new(connection: Connection) -> Self {
        Self {
            connection,
            _p: PhantomData,
        }
    }

    /// The underlying connection.
    pub fn connection(&self) -> &Connection {
        &self.connection
    }
}

#[async_trait::async_trait]
impl<P> Session<P> for QuicSession<P>
where
    P: Protocol<Error = Error> + 'static,
{
    type ClientLane = QuicTransport<P>;
    type ServiceLane = QuicServiceLane<P>;

    /// r[impl jetstream.session.capabilities]
    fn capabilities(&self) -> Capabilities {
        Capabilities::quic()
    }

    /// r[impl jetstream.session.identity.addressing]
    /// A certificate authenticates the peer once reached; the address is
    /// what reaches it, so both are reported.
    fn context(&self) -> Context {
        Context::new(
            Some(RemoteAddr::IpAddr(self.connection.remote_address().ip())),
            peer_from_connection(&self.connection),
        )
    }

    /// r[impl jetstream.lane.independence]
    /// Every lane is its own QUIC stream, so one lane's stalled frame
    /// does not delay another's.
    async fn open_lane(&self) -> Result<Self::ClientLane, SessionError> {
        let streams = self
            .connection
            .open_bi()
            .await
            .map_err(|err| SessionError::Transport(err.into_error()))?;

        Ok(QuicTransport::from(streams))
    }

    /// r[impl jetstream.session.symmetric]
    /// Either peer may open a lane on a QUIC connection.
    async fn accept_lane(&self) -> Result<Self::ServiceLane, SessionError> {
        let (send_stream, recv_stream) = self
            .connection
            .accept_bi()
            .await
            .map_err(|err| SessionError::Transport(err.into_error()))?;

        Ok(QuicServiceLane {
            send_stream: FramedWrite::new(send_stream, ServerCodec::new()),
            recv_stream: FramedRead::new(recv_stream, ServerCodec::new()),
            context: <Self as Session<P>>::context(self),
        })
    }

    /// r[impl jetstream.session.lifetime]
    /// Closing the connection resets every stream opened on it, so lanes
    /// terminate with the session and calls in flight on them fail
    /// rather than hang.
    async fn close(&self) {
        self.connection
            .close(VarInt::from_u32(SESSION_CLOSED), b"session closed");
    }
}

/// r[impl jetstream.session.datagrams]
/// QUIC datagrams belong to the connection, not to any stream on it.
#[async_trait::async_trait]
impl<P> Datagrams<P> for QuicSession<P>
where
    P: Protocol<Error = Error> + 'static,
{
    fn max_datagram_size(&self) -> Option<u32> {
        self.connection
            .max_datagram_size()
            .map(|size| size.min(u32::MAX as usize) as u32)
    }

    async fn send_datagram(
        &self,
        frame: Frame<P::Request>,
    ) -> Result<(), SessionError> {
        // r[impl jetstream.session.datagrams]
        // Rejected here rather than fragmented.
        check_datagram_size(&frame, self.max_datagram_size())?;

        let mut buf = Vec::with_capacity(frame.byte_size() as usize);
        frame
            .encode(&mut buf)
            .map_err(|err| SessionError::Transport(Error::from(err)))?;

        self.connection
            .send_datagram(Bytes::from(buf))
            .map_err(|err| SessionError::Transport(err.into_error()))
    }

    async fn recv_datagram(&self) -> Result<Frame<P::Response>, SessionError> {
        let datagram = self
            .connection
            .read_datagram()
            .await
            .map_err(|err| SessionError::Transport(err.into_error()))?;

        // r[impl jetstream.session.datagrams]
        // A datagram carries a complete frame or it is discarded.
        let mut reader = std::io::Cursor::new(datagram.as_ref());
        Frame::<P::Response>::decode(&mut reader)
            .map_err(|err| SessionError::Transport(Error::from(err)))
    }
}

/// The callee's end of a lane the peer opened.
///
/// r[impl jetstream.lane.definition]
/// One QUIC bidirectional stream, framed, satisfying
/// `ServiceTransport<P>`.
pub struct QuicServiceLane<P: Protocol> {
    send_stream: FramedWrite<SendStream, ServerCodec<P>>,
    recv_stream: FramedRead<RecvStream, ServerCodec<P>>,
    context: Context,
}

impl<P: Protocol> QuicServiceLane<P> {
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

impl<P: Protocol> Sink<Frame<P::Response>> for QuicServiceLane<P> {
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

impl<P: Protocol> Stream for QuicServiceLane<P> {
    type Item = Result<Frame<P::Request>, Error>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.get_mut().recv_stream.poll_next_unpin(cx)
    }
}

impl<P: Protocol> Contextual for QuicServiceLane<P> {
    fn context(&self) -> Context {
        self.context.clone()
    }
}

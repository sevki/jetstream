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
    task::{Context as TaskContext, Poll},
};

use futures::{Sink, SinkExt, Stream, StreamExt};
use iroh::{
    endpoint::{Connection, RecvStream, SendStream, VarInt},
    Endpoint,
};
use jetstream_rpc::{
    context::{Context, Contextual, NodeId},
    server::ServerCodec,
    session::{
        check_datagram_size, Capabilities, Datagrams, Session, SessionError,
    },
    Error, Frame, IntoError, Protocol,
};
use jetstream_wireformat::WireFormat;
use tokio_util::{
    bytes::Bytes,
    codec::{FramedRead, FramedWrite},
};

use crate::IrohTransport;

/// The code an iroh connection is closed with when its session closes.
const SESSION_CLOSED: u32 = 0;

/// A JetStream session over an iroh connection.
///
/// Cloning the session is cheap and every clone addresses the same
/// connection, so lanes can be opened from several tasks at once.
#[derive(Debug, Clone)]
pub struct IrohSession<P: Protocol> {
    connection: Connection,
    // Held for as long as the session is, for the same reason
    // `IrohTransport` holds them: iroh tears the connection down
    // ungracefully if the `Endpoint` is dropped while streams opened
    // from it are still in use.
    endpoint: Option<Endpoint>,
    _p: PhantomData<fn() -> P>,
}

impl<P: Protocol> IrohSession<P> {
    /// Wrap a connection whose endpoint the caller keeps alive.
    pub fn new(connection: Connection) -> Self {
        Self {
            connection,
            endpoint: None,
            _p: PhantomData,
        }
    }

    /// Wrap a connection and keep `endpoint` alive alongside it.
    pub fn new_owned(connection: Connection, endpoint: Endpoint) -> Self {
        Self {
            connection,
            endpoint: Some(endpoint),
            _p: PhantomData,
        }
    }

    /// The underlying connection.
    pub fn connection(&self) -> &Connection {
        &self.connection
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
    fn capabilities(&self) -> Capabilities {
        Capabilities::iroh()
    }

    /// r[impl jetstream.session.identity]
    /// iroh establishes the peer's public key during the handshake, so
    /// the identity is available without the peer asserting anything.
    fn context(&self) -> Context {
        Context::from(NodeId::from(self.connection.remote_id()))
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

        Ok(match &self.endpoint {
            Some(endpoint) => IrohTransport::new_owned(
                streams,
                self.connection.clone(),
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
            .connection
            .accept_bi()
            .await
            .map_err(|err| SessionError::Transport(err.into_error()))?;

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
        self.connection
            .close(VarInt::from_u32(SESSION_CLOSED), b"session closed");
    }
}

/// r[impl jetstream.session.datagrams]
/// iroh carries QUIC datagrams, which belong to the connection and not
/// to any stream on it.
#[async_trait::async_trait]
impl<P> Datagrams<P> for IrohSession<P>
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
        // A datagram carries a complete frame or it is discarded; there
        // is nothing to reassemble it from. Trailing bytes after the
        // frame mean it is not a complete frame either — decoding would
        // succeed on the prefix and hand back something the peer never
        // sent.
        let mut reader = std::io::Cursor::new(datagram.as_ref());
        let frame = Frame::<P::Response>::decode(&mut reader)
            .map_err(|err| SessionError::Transport(Error::from(err)))?;

        let consumed = reader.position() as usize;
        if consumed != datagram.len() {
            return Err(SessionError::Transport(Error::new(format!(
                "datagram has {} trailing bytes after the frame",
                datagram.len() - consumed
            ))));
        }

        Ok(frame)
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

//! The WebTransport session as a JetStream session.
//!
//! r[impl jetstream.session.conformance.webtransport]
//! The existing handler already satisfies the lane requirements on both
//! sides — `r[jetstream.webtransport.bidi.multi-stream]` is the accept
//! loop and `r[jetstream.webtransport.upstream-initiated]` is the open
//! path. This expresses them as the session trait rather than as a
//! bespoke path, so a caller reaches WebTransport the same way it
//! reaches iroh or QUIC.

use std::{
    marker::PhantomData,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context as TaskContext, Poll},
};

use bytes::Bytes;
use futures::{Sink, SinkExt, Stream, StreamExt};
use h3::quic;
use h3_webtransport::server::{
    AcceptedBi, WebTransportSession as H3WebTransportSession,
};
use jetstream_rpc::{
    client::ClientCodec,
    context::{Context, Contextual},
    server::ServerCodec,
    session::{Capabilities, Session, SessionError},
    Error, Frame, Protocol,
};
use tokio_util::codec::{FramedRead, FramedWrite};

/// The h3 connection a JetStream WebTransport session runs over.
type WtConnection = h3_quinn::Connection;

/// One WebTransport bidirectional stream — a lane.
type WtBidiStream = h3_webtransport::stream::BidiStream<
    <WtConnection as quic::OpenStreams<Bytes>>::BidiStream,
    Bytes,
>;

type WtSendStream = <WtBidiStream as quic::BidiStream<Bytes>>::SendStream;

type WtRecvStream = <WtBidiStream as quic::BidiStream<Bytes>>::RecvStream;

/// A JetStream session over a WebTransport session.
///
/// Cloning is cheap and every clone addresses the same WebTransport
/// session, so lanes can be opened from several tasks at once.
pub struct WebTransportSession<P: Protocol> {
    session: Arc<H3WebTransportSession<WtConnection, Bytes>>,
    context: Context,
    closed: Arc<AtomicBool>,
    _p: PhantomData<fn() -> P>,
}

impl<P: Protocol> Clone for WebTransportSession<P> {
    fn clone(&self) -> Self {
        Self {
            session: self.session.clone(),
            context: self.context.clone(),
            closed: self.closed.clone(),
            _p: PhantomData,
        }
    }
}

impl<P: Protocol> WebTransportSession<P> {
    /// Wrap an accepted WebTransport session.
    ///
    /// `context` is the peer identity the HTTP/3 handshake established —
    /// the client certificate, where there is one.
    pub fn new(
        session: Arc<H3WebTransportSession<WtConnection, Bytes>>,
        context: Context,
    ) -> Self {
        Self {
            session,
            context,
            closed: Arc::new(AtomicBool::new(false)),
            _p: PhantomData,
        }
    }

    /// The underlying WebTransport session.
    pub fn inner(&self) -> &Arc<H3WebTransportSession<WtConnection, Bytes>> {
        &self.session
    }

    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl<P> Session<P> for WebTransportSession<P>
where
    P: Protocol<Error = Error> + 'static,
{
    type ClientLane = WebTransportClientLane<P>;
    type ServiceLane = WebTransportServiceLane<P>;

    /// r[impl jetstream.session.capabilities]
    fn capabilities(&self) -> Capabilities {
        Capabilities::webtransport()
    }

    /// r[impl jetstream.session.identity]
    /// The identity the HTTP/3 handshake established, which for a
    /// WebTransport session is the client certificate where one was
    /// presented.
    fn context(&self) -> Context {
        self.context.clone()
    }

    /// r[impl jetstream.webtransport.upstream-initiated]
    /// r[impl jetstream.session.symmetric]
    /// The server opening a lane is the ordinary case here, not a
    /// special one: the end that opens a lane is the caller on it, so
    /// this lane is written as requests and read as responses.
    async fn open_lane(&self) -> Result<Self::ClientLane, SessionError> {
        if self.is_closed() {
            return Err(SessionError::Closed);
        }

        let stream = self
            .session
            .open_bi(self.session.session_id())
            .await
            .map_err(|err| {
                SessionError::Transport(Error::from(std::io::Error::other(
                    err.to_string(),
                )))
            })?;

        let (send, recv) = quic::BidiStream::split(stream);
        Ok(WebTransportClientLane {
            send_stream: FramedWrite::new(send, ClientCodec::default()),
            recv_stream: FramedRead::new(recv, ClientCodec::default()),
        })
    }

    /// r[impl jetstream.webtransport.bidi.multi-stream]
    /// r[impl jetstream.lane.independence]
    /// Each accepted lane is its own WebTransport stream. Streams the
    /// peer opens that are HTTP/3 requests rather than WebTransport
    /// streams are not lanes, and are skipped.
    async fn accept_lane(&self) -> Result<Self::ServiceLane, SessionError> {
        loop {
            if self.is_closed() {
                return Err(SessionError::Closed);
            }

            let accepted = self.session.accept_bi().await.map_err(|err| {
                SessionError::Transport(Error::from(std::io::Error::other(
                    err.to_string(),
                )))
            })?;

            match accepted {
                Some(AcceptedBi::BidiStream(_, stream)) => {
                    let (send, recv) = quic::BidiStream::split(stream);
                    return Ok(WebTransportServiceLane {
                        send_stream: FramedWrite::new(send, ServerCodec::new()),
                        recv_stream: FramedRead::new(recv, ServerCodec::new()),
                        context: self.context.clone(),
                    });
                }
                // An HTTP/3 request on the session is not a lane.
                Some(AcceptedBi::Request(..)) => continue,
                None => return Err(SessionError::Closed),
            }
        }
    }

    /// Stop opening and accepting lanes on this session.
    ///
    /// r[impl jetstream.session.lifetime]
    /// Partial: `h3-webtransport` 0.1 exposes no way to close a
    /// WebTransport session, so this marks the session closed — further
    /// opens and accepts fail with [`SessionError::Closed`] — but lanes
    /// already handed out are terminated by dropping them or by the
    /// underlying QUIC connection going away, not by this call. Said
    /// plainly rather than emulated: a lane here can outlive its
    /// session, which the other two bindings do not allow.
    async fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
    }
}

/// The caller's end of a lane this peer opened.
pub struct WebTransportClientLane<P: Protocol> {
    send_stream: FramedWrite<WtSendStream, ClientCodec<P>>,
    recv_stream: FramedRead<WtRecvStream, ClientCodec<P>>,
}

impl<P: Protocol> Sink<Frame<P::Request>> for WebTransportClientLane<P> {
    type Error = Error;

    fn poll_ready(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.get_mut().send_stream.poll_ready_unpin(cx)
    }

    fn start_send(
        self: Pin<&mut Self>,
        item: Frame<P::Request>,
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

impl<P: Protocol> Stream for WebTransportClientLane<P> {
    type Item = Result<Frame<P::Response>, Error>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.get_mut().recv_stream.poll_next_unpin(cx)
    }
}

/// The callee's end of a lane the peer opened.
pub struct WebTransportServiceLane<P: Protocol> {
    send_stream: FramedWrite<WtSendStream, ServerCodec<P>>,
    recv_stream: FramedRead<WtRecvStream, ServerCodec<P>>,
    context: Context,
}

impl<P: Protocol> Sink<Frame<P::Response>> for WebTransportServiceLane<P> {
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

impl<P: Protocol> Stream for WebTransportServiceLane<P> {
    type Item = Result<Frame<P::Request>, Error>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.get_mut().recv_stream.poll_next_unpin(cx)
    }
}

impl<P: Protocol> Contextual for WebTransportServiceLane<P> {
    fn context(&self) -> Context {
        self.context.clone()
    }
}

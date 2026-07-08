use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Sink, SinkExt, Stream, StreamExt};
use iroh::endpoint::{Connection, RecvStream, SendStream};
use jetstream_rpc::{client::ClientCodec, Error, Protocol};
use tokio_util::codec::{FramedRead, FramedWrite};

pub struct IrohTransport<P: Protocol> {
    send_stream: FramedWrite<SendStream, ClientCodec<P>>,
    recv_stream: FramedRead<RecvStream, ClientCodec<P>>,
    // Kept alive for as long as the transport is: iroh aborts the
    // connection ungracefully if the `Connection`/`Endpoint` are dropped
    // while streams opened from them are still in use.
    _keepalive: Option<(Connection, iroh::Endpoint)>,
}

impl<P: Protocol> From<(SendStream, RecvStream)> for IrohTransport<P> {
    fn from(value: (SendStream, RecvStream)) -> Self {
        let (send_stream, recv_stream) = value;
        let send_stream = FramedWrite::new(send_stream, ClientCodec::default());
        let recv_stream = FramedRead::new(recv_stream, ClientCodec::default());
        Self {
            send_stream,
            recv_stream,
            _keepalive: None,
        }
    }
}

impl<P: Protocol> IrohTransport<P> {
    /// Build a transport that keeps `connection` and `endpoint` alive for
    /// as long as the transport itself is alive. Use this whenever the
    /// caller doesn't otherwise hold on to the `Connection`/`Endpoint`
    /// (e.g. inside a helper function that would otherwise drop them on
    /// return).
    pub fn new_owned(
        streams: (SendStream, RecvStream),
        connection: Connection,
        endpoint: iroh::Endpoint,
    ) -> Self {
        let mut transport = Self::from(streams);
        transport._keepalive = Some((connection, endpoint));
        transport
    }
}

impl<P: Protocol> Sink<jetstream_rpc::Frame<P::Request>> for IrohTransport<P>
where
    Self: Unpin,
{
    type Error = Error;

    fn poll_ready(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.get_mut().send_stream.poll_ready_unpin(cx)
    }

    fn start_send(
        self: Pin<&mut Self>,
        item: jetstream_rpc::Frame<P::Request>,
    ) -> Result<(), Self::Error> {
        self.get_mut().send_stream.start_send_unpin(item)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.get_mut().send_stream.poll_flush_unpin(cx)
    }

    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.get_mut().send_stream.poll_close_unpin(cx)
    }
}

impl<P: Protocol> Stream for IrohTransport<P>
where
    Self: Unpin,
{
    type Item = Result<jetstream_rpc::Frame<P::Response>, Error>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.get_mut().recv_stream.poll_next_unpin(cx)
    }
}

use std::{
    io::{self, ErrorKind},
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Sink, Stream};
use jetstream_rpc::{Frame, Framer, Protocol};
use jetstream_wireformat::{
    wire_format_extensions::ConvertWireFormat, WireFormat,
};
use tungstenite::{Message, WebSocket};

pub struct WebSocketTransport<P: Protocol>(
    WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
    PhantomData<P>,
);

impl<P: Protocol>
    From<WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>>
    for WebSocketTransport<P>
{
    fn from(
        value: WebSocket<
            tungstenite::stream::MaybeTlsStream<std::net::TcpStream>,
        >,
    ) -> Self {
        Self(value, PhantomData)
    }
}

impl<P: Protocol> Sink<jetstream_rpc::Frame<P::Request>>
    for WebSocketTransport<P>
where
    Self: Unpin,
{
    type Error = io::Error;

    fn poll_ready(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        match self.0.can_write() {
            true => Poll::Ready(Ok(())),
            false => Poll::Pending,
        }
    }

    fn start_send(
        self: Pin<&mut Self>,
        item: jetstream_rpc::Frame<P::Request>,
    ) -> Result<(), Self::Error> {
        self.get_mut()
            .0
            .send(WebsocketFrame(item).into())
            .map_err(io::Error::other)?;
        Ok(())
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(self.get_mut().0.flush().map_err(io::Error::other))
    }

    fn poll_close(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(self.get_mut().0.close(None).map_err(io::Error::other))
    }
}

impl<P: Protocol> Stream for WebSocketTransport<P>
where
    Self: Unpin,
{
    type Item = Result<jetstream_rpc::Frame<P::Response>, io::Error>;

    fn poll_next(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match self.get_mut().0.read() {
            Ok(Message::Binary(bytes)) => {
                let mut reader = io::Cursor::new(bytes);
                let frame = Frame::<P::Response>::decode(&mut reader).unwrap();

                Poll::Ready(Some(Ok(frame)))
            }
            Err(e) => {
                eprintln!("Error reading from websocket: {:?}", e);
                Poll::Ready(None)
            }
            _ => {
                eprintln!("Unexpected message type from websocket");
                Poll::Ready(None)
            }
        }
    }
}

pub struct WebsocketFrame<F: Framer>(Frame<F>);

impl<F: Framer> From<WebsocketFrame<F>> for tungstenite::protocol::Message {
    fn from(value: WebsocketFrame<F>) -> Self {
        Message::Binary(value.0.to_bytes())
    }
}

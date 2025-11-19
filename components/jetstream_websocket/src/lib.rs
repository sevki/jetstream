#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub extern crate tokio_tungstenite;
use futures::{
    stream::{SplitSink, SplitStream},
    Sink, SinkExt, Stream, StreamExt,
};
use jetstream_rpc::{Error, Frame, Protocol};
use jetstream_wireformat::wire_format_extensions::ConvertWireFormat;
use std::{
    io,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream};

pub struct WebSocketTransport<P: Protocol>(
    SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    PhantomData<P>,
);

impl<P: Protocol + Unpin> From<WebSocketStream<MaybeTlsStream<TcpStream>>>
    for WebSocketTransport<P>
{
    fn from(ws: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Self {
        let (read, write) = ws.split();
        WebSocketTransport(read, write, PhantomData)
    }
}
impl<P: Protocol + Unpin> Sink<jetstream_rpc::Frame<P::Request>>
    for WebSocketTransport<P>
{
    type Error = Error;

    fn poll_ready(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.get_mut().0.poll_ready_unpin(cx).map_err(|e| {
            let err = io::Error::other(e);
            Error::from(err)
        })
    }

    fn start_send(
        self: Pin<&mut Self>,
        item: jetstream_rpc::Frame<P::Request>,
    ) -> Result<(), Self::Error> {
        self.get_mut()
            .0
            .start_send_unpin(Message::Binary(item.to_bytes()))
            .map_err(|e| Error::from(io::Error::other(e)))
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.get_mut()
            .0
            .poll_flush_unpin(cx)
            .map_err(|e| Error::from(io::Error::other(e)))
    }

    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.get_mut()
            .0
            .poll_close_unpin(cx)
            .map_err(|e| Error::from(io::Error::other(e)))
    }
}

impl<P: Protocol> Stream for WebSocketTransport<P>
where
    Self: Unpin,
{
    type Item = Result<jetstream_rpc::Frame<P::Response>, Error>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.get_mut().1.poll_next_unpin(cx).map(|opt| {
            opt.map(|res| match res {
                Ok(msg) => {
                    let data = msg.into_data();
                    Frame::<P::Response>::from_bytes(&data).map_err(Error::from)
                }
                Err(e) => Err(Error::from(io::Error::other(e))),
            })
        })
    }
}

use std::{pin::pin, str::FromStr};

use futures::{Sink, Stream};
use jetstream_wireformat::WireFormat;
use tokio_util::{
    bytes::{self, Buf, BufMut},
    codec::{Decoder, Encoder},
};

use crate::{
    context::{Context, Contextual},
    Error, Frame, Framer, Protocol, Version,
};

pub struct ServerCodec<P: Protocol> {
    _phantom: std::marker::PhantomData<P>,
}

impl<P: Protocol> ServerCodec<P> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<P: Protocol> Default for ServerCodec<P> {
    fn default() -> Self {
        Self::new()
    }
}

pub trait ServiceTransport<P: Protocol>:
    Sink<Frame<P::Response>, Error = P::Error>
    + Stream<Item = Result<Frame<P::Request>, P::Error>>
    + Send
    + Sync
    + Unpin
{
    fn context(&self) -> Context;
}

impl<P: Protocol, T> ServiceTransport<P> for T
where
    T: Sink<Frame<P::Response>, Error = P::Error>
        + Stream<Item = Result<Frame<P::Request>, P::Error>>
        + Send
        + Sync
        + Unpin,
    T: Contextual,
{
    fn context(&self) -> Context {
        <Self as Contextual>::context(self)
    }
}

impl<P> Decoder for ServerCodec<P>
where
    P: Protocol,
{
    type Error = Error;
    type Item = Frame<P::Request>;

    fn decode(
        &mut self,
        src: &mut bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        // check to see if you have at least 4 bytes to figure out the size
        if src.len() < 4 {
            src.reserve(4);
            return Ok(None);
        }
        let Some(mut bytz) = src.get(..4) else {
            return Ok(None);
        };

        let byte_size: u32 = WireFormat::decode(&mut bytz)?;
        if src.len() < byte_size as usize {
            src.reserve(byte_size as usize);
            return Ok(None);
        }

        let frame = Frame::<P::Request>::decode(&mut src.reader())?;
        Ok(Some(frame))
    }
}

impl<P> Encoder<Frame<P::Response>> for ServerCodec<P>
where
    P: Protocol,
{
    type Error = Error;

    fn encode(
        &mut self,
        item: Frame<P::Response>,
        dst: &mut bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        item.encode(&mut dst.writer())?;
        Ok(())
    }
}

#[trait_variant::make(Send + Sync + Sized)]
pub trait Server: Protocol + Send + Sync {
    /// Negotiate the protocol version to use.
    fn version(client_version: Version) -> jetstream_error::Result<Version> {
        // By default, accept any version that matches the major version of the server's protocol version.
        let server_version =
            Version::from_str(Self::VERSION).unwrap_or_else(|_| {
                panic!(
                    "Invalid version format for JetStream protocol: {}",
                    Self::VERSION
                )
            });
        match (client_version, server_version) {
            (Version::V9P2000L, Version::V9P2000L) => Ok(Version::V9P2000L),
            (Version::V9P2000, Version::V9P2000) => Ok(Version::V9P2000),
            (
                Version::JetStream {
                    name: client_name,
                    version: client_version,
                },
                Version::JetStream {
                    name: server_name,
                    version: server_version,
                },
            ) => {
                // compare versions of client and server and send the lowest version
                if client_name != server_name {
                    return Err(Error::new(format!(
                        "Incompatible protocol names: client={}, server={}",
                        client_name, server_name
                    )));
                }
                Ok(Version::JetStream {
                    name: server_name,
                    version: client_version.min(server_version),
                })
            }
            _ => Err(Error::new("Incompatible protocols".to_string())),
        }
    }
    /// The main RPC method that handles incoming requests and produces responses.
    async fn rpc(
        &mut self,
        context: Context,
        frame: Frame<Self::Request>,
    ) -> Result<Frame<Self::Response>, Self::Error>;

    /// r[impl jetstream.subscription.surface.declared]
    /// Whether a request of this message type opens a subscription. The
    /// declaration is the protocol's, not the call site's, so the
    /// dispatcher can route before it moves the frame.
    ///
    /// Defaults to "no streaming methods", which is every protocol
    /// written before this existed.
    fn is_streaming(_message_type: u8) -> bool {
        false
    }

    /// r[impl jetstream.subscription.overview]
    /// Serve a subscription: one request, many responses.
    ///
    /// r[impl jetstream.subscription.cancel]
    /// `cancel` is how the producer learns the subscriber has gone.
    /// Releasing only the delivery obligation is not enough — an
    /// inference or a build must be able to stop, not merely stop being
    /// listened to — so this is a parameter rather than something the
    /// dispatcher keeps to itself.
    ///
    /// Failures travel as items, per `r[jetstream.subscription.termination]`,
    /// which is also why the end can carry a value.
    ///
    /// r[impl jetstream.subscription.compat.rpc-layer]
    /// Defaulted, so every existing `Server` implementation compiles
    /// untouched: the breakage this change carries is on the client,
    /// where `RpcCall` resolves to one frame.
    async fn rpc_stream(
        &mut self,
        context: Context,
        frame: Frame<Self::Request>,
        cancel: tokio_util::sync::CancellationToken,
    ) -> ResponseStream<Self>
    where
        // The served stream is boxed and owned, so what it carries must
        // outlive the call that made it. Every real protocol's messages
        // are owned values, so this costs nothing but has to be said —
        // the same bound `Datagrams` needs, for the same reason.
        Self::Response: 'static,
        Self::Error: 'static,
    {
        let _ = (context, frame, cancel);
        // A bare async block, not a normal body: `#[trait_variant::make]`
        // rewrites `async fn` into `fn -> impl Future`, so the body must
        // *be* the future. The two natural spellings fail with errors
        // that point elsewhere entirely. An *implementation* of this
        // method writes an ordinary body, since the macro rewrites the
        // trait and not its impls.
        async move { Box::pin(futures::stream::empty()) as ResponseStream<Self> }
    }
}

/// The items a served subscription yields.
pub type ResponseStream<P> = std::pin::Pin<
    Box<
        dyn futures::Stream<
                Item = Result<
                    Frame<<P as Protocol>::Response>,
                    <P as Protocol>::Error,
                >,
            > + Send,
    >,
>;

pub async fn run<T, P>(p: &mut P, mut stream: T) -> Result<(), P::Error>
where
    T: ServiceTransport<P>,
    P: Server,
    P::Response: 'static,
    P::Error: 'static,
{
    use futures::{SinkExt, StreamExt};
    let mut a = pin!(p);
    while let Some(Ok(frame)) = stream.next().await {
        let context = stream.context();
        // r[impl jetstream.subscription.surface.declared]
        // Routed on the declared message type, before the frame moves.
        if P::is_streaming(frame.msg.message_type()) {
            // r[impl jetstream.subscription.cancel]
            // The token the producer watches, cancelled when the served
            // stream ends under it.
            let cancel = tokio_util::sync::CancellationToken::new();
            let mut items = a.rpc_stream(context, frame, cancel.clone()).await;
            while let Some(item) = items.next().await {
                stream.send(item?).await?;
            }
            cancel.cancel();
        } else {
            stream.send(a.rpc(context, frame).await?).await?
        }
    }
    Ok(())
}

use std::pin::pin;

use crate::{Error, Frame, Protocol};
use futures::{Sink, Stream};
use jetstream_wireformat::WireFormat;
use tokio_util::{
    bytes::{self, Buf, BufMut},
    codec::{Decoder, Encoder},
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
}

impl<P: Protocol, T> ServiceTransport<P> for T where
    T: Sink<Frame<P::Response>, Error = P::Error>
        + Stream<Item = Result<Frame<P::Request>, P::Error>>
        + Send
        + Sync
        + Unpin
{
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

        Frame::<P::Request>::decode(&mut src.reader())
            .map(Some)
            .map_err(|_| Error::Custom("()".to_string()))
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
        item.encode(&mut dst.writer())
            .map_err(|_| Error::Custom("()".to_string()))
            .map(|_| ())
    }
}

pub async fn run<T, P>(p: &mut P, mut stream: T) -> Result<(), P::Error>
where
    T: ServiceTransport<P>,
    P: Protocol,
{
    use futures::{SinkExt, StreamExt};
    let mut a = pin!(p);
    while let Some(Ok(frame)) = stream.next().await {
        stream.send(a.rpc(frame).await?).await?
    }
    Ok(())
}

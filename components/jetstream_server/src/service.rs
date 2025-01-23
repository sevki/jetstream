use {
    jetstream_rpc::{Error, Frame, Protocol, ServiceTransport},
    jetstream_wireformat::WireFormat,
    std::pin::pin,
    tokio_util::{
        bytes::{self, Buf, BufMut},
        codec::{Decoder, Encoder},
    },
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

impl<P> Decoder for ServerCodec<P>
where
    P: Protocol,
{
    type Error = Error;
    type Item = Frame<P::Request>;

    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Frame::<P::Request>::decode(&mut src.reader())
            .map(Some)
            .map_err(Error::Io)
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
            .map_err(Error::Io)
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
        let res = a.rpc(frame).await?;
        stream.send(res).await?
    }
    Ok(())
}

use std::{io, marker::PhantomData};

use crate::{context::NodeAddr, Error, Frame, Protocol};
use futures::{
    stream::{SplitSink, SplitStream},
    Sink, Stream, StreamExt,
};
use jetstream_wireformat::WireFormat;
use tokio_util::{
    bytes::{self, Buf, BufMut},
    codec::{Decoder, Encoder},
};

pub struct ClientCodec<P>
where
    P: Protocol,
{
    _p: std::marker::PhantomData<P>,
}

impl<P: Protocol> Encoder<Frame<P::Request>> for ClientCodec<P> {
    type Error = Error;

    fn encode(
        &mut self,
        item: Frame<P::Request>,
        dst: &mut bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        WireFormat::encode(&item, &mut dst.writer())?;
        Ok(())
    }
}

impl<P: Protocol> Decoder for ClientCodec<P> {
    type Error = Error;
    type Item = Frame<P::Response>;

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
        let frame = Frame::<P::Response>::decode(&mut src.reader())?;
        Ok(Some(frame))
    }
}

impl<P> Default for ClientCodec<P>
where
    P: Protocol,
{
    fn default() -> Self {
        Self {
            _p: std::marker::PhantomData,
        }
    }
}

pub trait ClientTransport<P: Protocol>:
    Sink<Frame<P::Request>, Error = Error>
    + Stream<Item = Result<Frame<P::Response>, Error>>
    + Send
    + Sync
    + Unpin
{
}

impl<P: Protocol, T> ClientTransport<P> for T
where
    Self: Sized,
    T: Sink<Frame<P::Request>, Error = Error>
        + Stream<Item = Result<Frame<P::Response>, Error>>
        + Send
        + Sync
        + Unpin,
{
}

pub trait Channel<P: Protocol>: Unpin + Sized {
    fn split(self) -> (SplitSink<Self, Frame<P::Request>>, SplitStream<Self>);
}

impl<P, T> Channel<P> for T
where
    P: Protocol,
    T: ClientTransport<P> + Unpin + Sized,
{
    fn split(
        self,
    ) -> (
        SplitSink<Self, Frame<<P as Protocol>::Request>>,
        SplitStream<Self>,
    ) {
        StreamExt::split(self)
    }
}

#[derive(Debug)]
pub struct ClientBuilder<P: Protocol> {
    node_addr: NodeAddr,
    _phantom: PhantomData<P>,
}

impl<P: Protocol> WireFormat for ClientBuilder<P> {
    fn byte_size(&self) -> u32 {
        P::VERSION.to_string().byte_size() + self.node_addr.byte_size()
    }

    fn encode<W: io::Write>(&self, writer: &mut W) -> io::Result<()>
    where
        Self: Sized,
    {
        P::VERSION.to_string().encode(writer)?;
        self.node_addr.encode(writer)
    }

    fn decode<R: io::Read>(reader: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let version = String::decode(reader)?;
        if version != P::VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "version mismatch",
            ));
        }
        let node_addr = NodeAddr::decode(reader)?;
        Ok(ClientBuilder {
            node_addr,
            _phantom: PhantomData,
        })
    }
}

pub fn client_builder<P: Protocol>(
    addr: impl Into<NodeAddr>,
) -> ClientBuilder<P> {
    ClientBuilder {
        node_addr: addr.into(),
        _phantom: PhantomData,
    }
}

impl<P: Protocol> From<(P, NodeAddr)> for ClientBuilder<P> {
    fn from(value: (P, NodeAddr)) -> Self {
        ClientBuilder {
            node_addr: value.1,
            _phantom: PhantomData,
        }
    }
}

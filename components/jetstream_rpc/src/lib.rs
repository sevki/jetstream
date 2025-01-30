#![doc(html_logo_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png")]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
//! # JetStream Rpc
//! Defines Rpc primitives for JetStream.
//! Of note is the `Protocol` trait which is meant to be used with the `service` attribute macro.
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use {
    futures::{Sink, Stream},
    jetstream_wireformat::WireFormat,
    std::{
        io::{self, ErrorKind, Read, Write},
        mem,
    },
};

/// A trait representing a message that can be encoded and decoded.
pub trait Message: WireFormat + Send + Sync {}

#[repr(transparent)]
pub struct Tag(u16);

impl From<u16> for Tag {
    fn from(tag: u16) -> Self {
        Self(tag)
    }
}

pub struct Context<T: WireFormat> {
    pub tag: Tag,
    pub msg: T,
}

pub trait FromContext<T: WireFormat> {
    fn from_context(ctx: Context<T>) -> Self;
}

impl<T: WireFormat> FromContext<T> for T {
    fn from_context(ctx: Context<T>) -> Self {
        ctx.msg
    }
}

impl<T: WireFormat> FromContext<T> for Tag {
    fn from_context(ctx: Context<T>) -> Self {
        ctx.tag
    }
}

pub trait Handler<T: WireFormat> {
    fn call(self, context: Context<T>);
}

/// Defines the request and response types for the JetStream protocol.
#[trait_variant::make(Send + Sync + Sized)]
pub trait Protocol: Send + Sync {
    type Request: Framer;
    type Response: Framer;
    type Error: std::error::Error + Send + Sync + 'static;
    const VERSION: &'static str;
    async fn rpc(
        &mut self,
        frame: Frame<Self::Request>,
    ) -> Result<Frame<Self::Response>, Self::Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("generic error: {0}")]
    Generic(#[from] Box<dyn std::error::Error + Send + Sync>),
    #[error("{0}")]
    Custom(String),
}

pub struct Frame<T: Framer> {
    pub tag: u16,
    pub msg: T,
}

impl<T: Framer> From<(u16, T)> for Frame<T> {
    fn from((tag, msg): (u16, T)) -> Self {
        Self { tag, msg }
    }
}

impl<T: Framer> WireFormat for Frame<T> {
    fn byte_size(&self) -> u32 {
        let msg_size = self.msg.byte_size();
        // size + type + tag + message size
        (mem::size_of::<u32>() + mem::size_of::<u8>() + mem::size_of::<u16>()) as u32 + msg_size
    }

    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.byte_size().encode(writer).expect("byte_size");

        let ty = self.msg.message_type();

        ty.encode(writer).expect("ty");
        self.tag.encode(writer).expect("tag");

        self.msg.encode(writer).expect("msg");

        Ok(())
    }

    fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
        let byte_size: u32 = WireFormat::decode(reader)?;

        // byte_size includes the size of byte_size so remove that from the
        // expected length of the message.  Also make sure that byte_size is at least
        // that long to begin with.
        if byte_size < mem::size_of::<u32>() as u32 {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("byte_size(= {}) is less than 4 bytes", byte_size),
            ));
        }
        let reader = &mut reader.take((byte_size - mem::size_of::<u32>() as u32) as u64);

        let mut ty = [0u8];
        reader.read_exact(&mut ty).expect("ty");

        let tag: u16 = WireFormat::decode(reader).expect("tag");
        let msg = T::decode(reader, ty[0]).expect("msg");

        Ok(Frame { tag, msg })
    }
}

pub trait Framer: Sized + Send + Sync {
    fn message_type(&self) -> u8;
    /// Returns the number of bytes necessary to fully encode `self`.
    fn byte_size(&self) -> u32;

    /// Encodes `self` into `writer`.
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()>;

    /// Decodes `Self` from `reader`.
    fn decode<R: Read>(reader: &mut R, ty: u8) -> io::Result<Self>;
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

pub trait ClientTransport<P: Protocol>:
    Sink<Frame<P::Request>, Error = std::io::Error>
    + Stream<Item = Result<Frame<P::Response>, std::io::Error>>
    + Send
    + Sync
    + Unpin
{
}

impl<P: Protocol, T> ClientTransport<P> for T
where
    Self: Sized,
    T: Sink<Frame<P::Request>, Error = std::io::Error>
        + Stream<Item = Result<Frame<P::Response>, std::io::Error>>
        + Send
        + Sync
        + Unpin,
{
}

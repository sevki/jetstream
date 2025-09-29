#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
//! # JetStream Rpc
//! Defines Rpc primitives for JetStream.
//! Of note is the `Protocol` trait which is meant to be used with the `service` attribute macro.
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use std::{
    io::{self, ErrorKind, Read, Write},
    mem,
};

use jetstream_wireformat::WireFormat;
// Re-export codecs
pub use tokio_util::codec::{Decoder, Encoder, Framed};

extern crate tokio_util;

pub mod client;
pub mod server;

/// A trait representing a message that can be encoded and decoded.
#[cfg(not(target_arch = "wasm32"))]
pub trait Message: WireFormat + Sync {}

/// A trait representing a message that can be encoded and decoded.
/// WebAssembly doesn't fully support Send+Sync, so we don't require those.
#[cfg(target_arch = "wasm32")]
pub trait Message: WireFormat {}

#[repr(transparent)]
pub struct Tag(u16);

impl From<u16> for Tag {
    fn from(tag: u16) -> Self {
        Self(tag)
    }
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
    #[error("invalid response")]
    InvalidResponse,
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
        (mem::size_of::<u32>() + mem::size_of::<u8>() + mem::size_of::<u16>())
            as u32
            + msg_size
    }

    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.byte_size().encode(writer)?;

        let ty = self.msg.message_type();

        ty.encode(writer)?;
        self.tag.encode(writer)?;

        self.msg.encode(writer)?;

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
                format!("byte_size(= {byte_size}) is less than 4 bytes"),
            ));
        }
        let reader =
            &mut reader.take((byte_size - mem::size_of::<u32>() as u32) as u64);

        let mut ty = [0u8];
        reader.read_exact(&mut ty)?;

        let tag: u16 = WireFormat::decode(reader)?;
        let msg = T::decode(reader, ty[0])?;

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

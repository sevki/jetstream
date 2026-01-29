#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
//! # JetStream Rpc
//! Defines Rpc primitives for JetStream.
//! Of note is the `Protocol` trait which is meant to be used with the `service` attribute macro.
#![cfg_attr(docsrs, feature(doc_cfg))]

use std::str::FromStr;
pub mod client;
pub mod context;
pub mod framer;
pub mod server;
pub use jetstream_error::IntoError;
use jetstream_wireformat::WireFormat;
// Re-export codecs
extern crate tokio_util;
pub use tokio_util::codec::{Decoder, Encoder, Framed};
mod dynamic;
pub use dynamic::AnyServer;
mod constants;
pub use constants::*;
mod tag;
pub use tag::*;
mod call;
pub use call::*;
mod mux;
pub use mux::*;
pub enum Encoding {
    JetStream,
    Json,
    Xml,
}

#[derive(Debug, thiserror::Error)]
pub enum EncodingError {
    #[error("Invalid encoding")]
    InvalidEncoding,
}

impl FromStr for Encoding {
    type Err = EncodingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            MIMETYPE_JSON => Ok(Encoding::Json),
            MIMETYPE_XML => Ok(Encoding::Xml),
            MIMETYPE_JETSTREAM => Ok(Encoding::JetStream),
            _ => Err(EncodingError::InvalidEncoding),
        }
    }
}

pub use constants::HEADER_KEY_JETSTREAM_PROTO;
pub use framer::*;

/// A trait representing a message that can be encoded and decoded.
#[cfg(native)]
pub trait Message: WireFormat + Sync {}

/// A trait representing a message that can be encoded and decoded.
/// WebAssembly doesn't fully support Send+Sync, so we don't require those.
#[cfg(target_arch = "wasm32")]
pub trait Message: WireFormat {}

/// Defines the request and response types for the JetStream protocol.
#[trait_variant::make(Send + Sync + Sized)]
pub trait Protocol: Send + Sync {
    type Request: Framer;
    type Response: Framer;
    // r[impl jetstream.error.type]
    // r[verify jetstream.error.type]
    type Error: IntoError;
    const VERSION: &'static str;
}

pub type Error = jetstream_error::Error;

const TLERROR: u8 = 6;
pub const RLERROR: u8 = TLERROR + 1;

pub const RJETSTREAMERROR: u8 = TLERROR - 1;

pub struct ErrorFrame(Error);

impl Framer for ErrorFrame {
    fn message_type(&self) -> u8 {
        RJETSTREAMERROR
    }

    fn byte_size(&self) -> u32 {
        self.0.byte_size()
    }

    fn encode<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.encode(writer)
    }

    fn decode<R: std::io::Read>(
        reader: &mut R,
        ty: u8,
    ) -> std::io::Result<Self> {
        match ty {
            RJETSTREAMERROR => {
                let err = Error::decode(reader)?;
                Ok(ErrorFrame(err))
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unknown message type: {}", ty),
            )),
        }
    }
}

impl From<Error> for ErrorFrame {
    fn from(err: Error) -> Self {
        ErrorFrame(err)
    }
}

impl From<ErrorFrame> for Error {
    fn from(frame: ErrorFrame) -> Self {
        frame.0
    }
}

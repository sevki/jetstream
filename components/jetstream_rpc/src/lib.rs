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
pub use dynamic::*;
mod constants;
pub use constants::*;

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
    // r[impl jetstream.error.type]
    // r[verify jetstream.error.type]
    type Error: IntoError;
    const VERSION: &'static str;
    async fn rpc(
        &mut self,
        context: context::Context,
        frame: Frame<Self::Request>,
    ) -> Result<Frame<Self::Response>, Self::Error>;
}

pub type Error = jetstream_error::Error;

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

extern crate tokio_util;
mod any_server;
mod call;
pub mod client;
mod constants;
pub mod context;
mod error;
pub mod framer;
mod mux;
pub mod server;
mod tag;
mod version;
pub use any_server::AnyServer;
pub use call::*;
pub use constants::*;
pub use error::*;
pub use jetstream_error::IntoError;
use jetstream_wireformat::WireFormat;
pub use mux::*;
use std::str::FromStr;
pub use tag::*;
pub use tokio_util::codec::{Decoder, Encoder, Framed};
pub use version::*;

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
    // r[impl jetstream.error.v2.into-error]
    type Error: IntoError;
    const VERSION: &'static str;
    const NAME: &'static str;
}

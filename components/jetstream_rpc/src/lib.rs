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

use std::io::{self};
pub mod client;
pub mod context;
pub mod framer;
pub mod server;
use jetstream_wireformat::WireFormat;
// Re-export codecs
extern crate tokio_util;
pub use tokio_util::codec::{Decoder, Encoder, Framed};

pub use framer::*;

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
        context: context::Context,
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

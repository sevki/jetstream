#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
//! # JetStream Rpc
//! Defines Rpc primitives for JetStream.
//! Of note is the `Protocol` trait which is meant to be used with the `service` attribute macro.
//!
//! [`client::ClientTransport`] and [`server::ServiceTransport`] describe
//! one ordered frame sequence. [`session`] describes the thing that can
//! open another one: a [`session::Session`] is the association with a
//! peer, and a *lane* is one sequence obtained from it — which is what
//! those two traits already are. See the guide at
//! <https://sevki.github.io/jetstream/sessions.html>.
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
mod router;
pub mod server;
pub mod session;
pub mod subscription;
mod tag;
mod version;
use std::str::FromStr;

pub use any_server::AnyServer;
pub use call::*;
pub use constants::*;
pub use error::*;
pub use jetstream_error::IntoError;
use jetstream_wireformat::WireFormat;
pub use mux::*;
pub use router::*;
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

    /// r[impl jetstream.subscription.cancel]
    /// The cancellation request for a subscription on this lane, if this
    /// protocol has subscriptions to cancel.
    ///
    /// The client needs to *build* a request it cannot name: `Mux` is
    /// generic over the protocol, and `subscription::Tcancel` is a
    /// payload, not a `Request`. Only the protocol knows how one is
    /// carried in its own request type, so it says here.
    ///
    /// r[impl jetstream.subscription.compat.rpc-layer]
    /// Defaulted to `None` — a protocol with no streaming methods has
    /// nothing to cancel, and every protocol written before this existed
    /// compiles untouched.
    ///
    /// `binding` is zero on the subscription's own lane, where `oldtag`
    /// is unambiguous; see `subscription::Tcancel`.
    fn tcancel(_oldtag: u16, _binding: u64) -> Option<Self::Request> {
        None
    }
}

// const _: () = {
//     let _: HashMap<String, Box<dyn Protocol>> = HashMap::new();

//     ()
// };

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![cfg_attr(docsrs, feature(doc_cfg))]
//! # JetStream
//! JetStream, is a collection of crates that provide a set of tools to build distributed systems.
//! It started it's life off in the CrosVM project, and has since been extracted into it's own project.
//! For more information please see the [JetStream Book](https://sevki.github.io/jetstream)
pub mod prelude {
    pub extern crate async_trait;
    pub extern crate futures;
    pub extern crate jetstream_macros;
    pub extern crate jetstream_rpc;
    pub extern crate jetstream_wireformat;
    pub extern crate lazy_static;
    pub extern crate trait_variant;

    pub use async_trait::async_trait;
    pub use jetstream_macros::{service, JetStreamWireFormat};
    pub use jetstream_rpc::{
        client, client::ClientTransport, context::Context, server, Error,
        Frame, Framed, Framer, Message, Protocol, Tag,
    };
    pub use jetstream_wireformat::{Data, WireFormat};
    pub use lazy_static::*;
    pub use trait_variant::make;

    #[cfg(feature = "tracing")]
    pub extern crate tracing;
    #[cfg(feature = "tracing")]
    pub use tracing::*;
    #[cfg(feature = "tracing")]
    pub extern crate tracing_subscriber;
}

#[cfg(feature = "9p")]
pub mod p9 {
    extern crate jetstream_9p;
    pub use jetstream_9p::*;
}

#[cfg(feature = "websocket")]
pub mod websocket {
    extern crate jetstream_websocket;
    pub use jetstream_websocket::*;
}

#[cfg(feature = "quic")]
pub mod quic {
    extern crate jetstream_quic;
    pub use jetstream_quic::*;
}

#[cfg(feature = "iroh")]
pub mod iroh {
    extern crate jetstream_iroh;
    pub use jetstream_iroh::*;
}

#[cfg(feature = "cloudflare")]
pub mod cloudflare {
    extern crate jetstream_cloudflare;
    pub use jetstream_cloudflare::*;
}

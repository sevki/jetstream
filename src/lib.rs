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
    pub extern crate jetstream_error;
    pub extern crate jetstream_macros;
    pub extern crate jetstream_rpc;
    pub extern crate jetstream_wireformat;
    pub extern crate lazy_static;
    pub extern crate trait_variant;

    pub use async_trait::async_trait;
    pub use jetstream_error::*;
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

/// Creates a newtype wrapper around a prost Message type that implements WireFormat.
///
/// # Examples
///
/// ```rust
/// use jetstream::prelude::*;
/// use jetstream::prost_wireformat;
/// use prost::Message;
///
/// // Define a simple prost message for testing
/// #[derive(Clone, PartialEq, Message)]
/// pub struct TestMessage {
///     #[prost(uint64, tag = "1")]
///     pub id: u64,
///     #[prost(string, tag = "2")]
///     pub name: String,
///     #[prost(bytes = "vec", tag = "3")]
///     pub data: Vec<u8>,
/// }
///
/// // Public struct, private inner field and accessors
/// prost_wireformat!(pub TestMessage as MyMessage);
///
/// // Public struct with public inner field and accessors
/// prost_wireformat!(pub TestMessage as pub YourMessage);
///
/// // Crate-visible struct with public inner access
/// prost_wireformat!(pub(crate) TestMessage as pub OurMessage);
///
/// // With derives - adds Clone and Debug to the wrapper
/// prost_wireformat!(pub TestMessage as CloneableMessage, derive(Clone, Debug));
/// ```
#[macro_export]
macro_rules! prost_wireformat {
    // With derives: prost_wireformat!(pub TestMessage as MyMessage, derives(Clone, Debug));
    ($wrapper_vis:vis $wrapped_type:ty as $vis:vis $new_type:ident, derive($($derives:path),* $(,)?)) => {
        #[derive($($derives),*)]
        $wrapper_vis struct $new_type($vis $wrapped_type);

        $crate::prost_wireformat!(@impl $wrapped_type, $vis, $new_type);
    };

    // Without derives: prost_wireformat!(pub TestMessage as MyMessage);
    ($wrapper_vis:vis $wrapped_type:ty as $vis:vis $new_type:ident) => {
        $wrapper_vis struct $new_type($vis $wrapped_type);

        $crate::prost_wireformat!(@impl $wrapped_type, $vis, $new_type);
    };

    // Internal: shared implementation
    (@impl $wrapped_type:ty, $vis:vis, $new_type:ident) => {
        impl $new_type {
            $vis fn new(inner: $wrapped_type) -> Self {
                Self(inner)
            }

            $vis fn into_inner(self) -> $wrapped_type {
                self.0
            }

            $vis fn inner(&self) -> &$wrapped_type {
                &self.0
            }
        }

        impl $crate::prelude::WireFormat for $new_type {
            fn byte_size(&self) -> u32 {
                std::mem::size_of::<usize>() as u32
                    + self.0.encoded_len() as u32
            }

            fn encode<W: std::io::Write>(
                &self,
                writer: &mut W,
            ) -> std::io::Result<()> {
                let len = self.0.encoded_len();
                len.encode(writer)?;
                let mut buf = Vec::with_capacity(len);
                self.0
                    .encode(&mut buf)
                    .map_err(std::io::Error::other)?;
                writer.write_all(&buf)
            }

            fn decode<R: std::io::Read>(
                reader: &mut R,
            ) -> std::io::Result<Self> {
                let len = usize::decode(reader)?;
                let mut bytes = vec![0; len];
                reader.read_exact(&mut bytes)?;
                <$wrapped_type as prost::Message>::decode(bytes.as_slice())
                    .map($new_type)
                    .map_err(std::io::Error::other)
            }
        }
    };
}

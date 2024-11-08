#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

pub mod prelude {
    pub use trait_variant;

    pub use jetstream_derive::{service, JetStreamWireFormat};

    pub use jetstream_rpc::{Error, Message, Protocol, Service};

    pub use jetstream_wireformat::{Data, WireFormat};

    #[cfg(feature = "9p")]
    pub use jetstream_9p::*;

    #[cfg(feature = "client")]
    pub use jetstream_client::*;

    #[cfg(feature = "server")]
    pub use jetstream_server::*;

    #[cfg(feature = "ufs")]
    pub use jetstream_ufs::*;
}

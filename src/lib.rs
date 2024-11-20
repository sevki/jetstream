#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
//! # JetStream
//! JetStream, is a collection of crates that provide a set of tools to build distributed systems.
//! It started it's life off in the CrosVM project, and has since been extracted into it's own project.
//! For more information please see the [JetStream Book](https://sevki.github.io/jetstream)
pub mod prelude {

    pub use jetstream_macros::{service, JetStreamWireFormat};

    pub use jetstream_rpc::{Error, Message, Protocol, Service};

    pub use jetstream_wireformat::{Data, WireFormat};

    pub use trait_variant;

    #[cfg(feature = "9p")]
    pub mod p9 {
        pub use jetstream_9p::*;
    }

    #[cfg(feature = "client")]
    pub mod client {
        pub use jetstream_client::*;
    }

    #[cfg(feature = "server")]
    pub mod server {
        pub use jetstream_server::*;
    }

    #[cfg(feature = "distributed")]
    pub mod distributed {
        pub use jetstream_distributed::*;
    }
}

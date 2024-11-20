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
    /// Re-export the `service` and `JetStreamWireFormat` macros.
    pub use jetstream_macros::{service, JetStreamWireFormat};
    /// RPC related traits and types.
    pub use jetstream_rpc::{Error, Message, Protocol, Service};
    /// `Data` and `WireFormat`.
    pub use jetstream_wireformat::{Data, WireFormat};
    /// Re-export trait variant.
    pub use trait_variant;

    #[cfg(feature = "9p")]
    pub use jetstream_9p::*;

    #[cfg(feature = "client")]
    pub use jetstream_client::*;

    #[cfg(feature = "server")]
    pub use jetstream_server::*;

    #[cfg(feature = "ufs")]
    pub use jetstream_ufs::*;

    /// JetStream Cluster is under development.
    #[cfg(feature = "cluster")]
    pub use jetstream_cluster::*;
}

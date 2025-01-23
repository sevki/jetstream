#![deny(missing_docs)]
#![deny(clippy::missing_safety_doc)]
#![doc(html_logo_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png")]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
//! # JetStream Distributed
//! JetStream Distributed is a collection of primitives for building distributed systems with JetStream.
pub mod access_control;
pub mod cluster;
pub mod coordinate;
pub mod placement;

/// Error type
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Failed to join node to the cluster
    #[error("Failed to join node")]
    JoinFailed,
}
/// Result type for JetStream Cluster operations
pub type Result<T> = std::result::Result<T, Error>;

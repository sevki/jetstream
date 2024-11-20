//! Cluster provides memebership primitives for JetStream.
use jetstream_rpc::{Protocol, Service};
use okid::OkId;

use super::{coordinate::Coordinate, Result};

/// Cluster trait
#[trait_variant::make(Send+Sync)]
pub trait Cluster {
    /// Join a cluster
    fn join(&self, node: impl IntoNode) -> Result<()>;
    /// Leave a cluster
    fn leave(&self, node: impl IntoNode) -> Result<()>;
}

/// A Node trait
#[trait_variant::make(Send+Sync)]
pub trait Node {
    /// ID, this is a unique identifier for the node.
    fn id(&self) -> NodeId;
    /// Coordinate
    fn coordinate(&self) -> Result<Coordinate>;
    /// dial
    async fn dial<P: Protocol>(&self) -> Result<impl Service<P>>;
}

/// IntoNode trait
pub trait IntoNode {
    /// Convert into a node
    fn into_node(self) -> impl Node;
}

/// Node ID
#[derive(Clone, Debug)]
pub enum NodeId {
    /// Persistent node ID
    /// This is a unique identifier for the node, usually the XOR of servers MAC addresses.
    Persistent(OkId),
    /// Transient node ID
    /// This is a unique identifier for the node, the fingerprint of the public key.
    Transient(OkId),
}

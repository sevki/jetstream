//! Cluster module

use okid::OkId;

use super::{coordinate::Coordinate, Result};

/// Cluster trait
#[trait_variant::make(Send+Sync)]
pub trait Cluster {
    /// Join a cluster
    fn join(&self, node: impl Node) -> Result<()>;
    /// Leave a cluster
    fn leave(&self, node: impl Node) -> Result<()>;
}

/// A Node trait
pub trait Node {
    /// ID, this is a unique identifier for the node.

    /// Coordinate
    fn coordinate(&self) -> Result<Coordinate>;
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

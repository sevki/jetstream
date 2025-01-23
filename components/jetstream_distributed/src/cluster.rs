//! Cluster provides memebership primitives for JetStream.
use {jetstream_rpc::Protocol, mac_address::MacAddressIterator};

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
    async fn dial<P: Protocol>(&self) -> Result<impl Protocol>;
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
    Persistent(PersistentId),
    /// Transient node ID
    /// This is a unique identifier for the node, the fingerprint of the public key.
    Transient(TransientId),
}

/// Persistent ID
/// This is a unique identifier for the node, usually the XOR of servers MAC addresses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistentId([u8; 6]);

impl From<MacAddressIterator> for PersistentId {
    fn from(iter: MacAddressIterator) -> Self {
        let mut iter = iter.into_iter();
        let mut bytes_now = iter.next().unwrap().bytes();
        for bytes in iter {
            bytes_now
                .iter_mut()
                .zip(bytes.bytes().iter())
                .for_each(|(a, b)| *a ^= b);
        }
        PersistentId(bytes_now)
    }
}

/// Transient ID
/// This is a unique identifier for the node, the fingerprint of the public key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransientId([u8; 32]);

use sha2::Digest;
impl From<sha2::Sha256> for TransientId {
    fn from(digest: sha2::Sha256) -> Self {
        let mut bytes = [0; 32];
        bytes.copy_from_slice(digest.finalize().as_slice());
        TransientId(bytes)
    }
}

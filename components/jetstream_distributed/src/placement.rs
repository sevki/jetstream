//! Placement module is used to determine the placement of the object,
//! and later to locate the object.

use crate::cluster::IntoNode;

/// Placement trait.
pub trait Placement {
    /// Returns the placement of the object.
    fn map<V>(&self, value: &V) -> Vec<impl IntoNode>
    where
        V: Ord + Eq;
    /// Returns the placement of the object.
    fn locate<V>(&self, value: &V) -> Vec<impl IntoNode>
    where
        V: Ord + Eq;
}

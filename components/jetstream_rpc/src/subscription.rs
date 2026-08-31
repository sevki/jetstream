//! Wire pieces for subscriptions.
//!
//! r[impl jetstream.subscription.overview]
//! A subscription is a streaming response: one request, many responses
//! sharing its tag, terminated explicitly. This module carries the three
//! things that shape needs on the wire and nothing else — the terminator,
//! cancellation, and the endpoint identifier. The client and server
//! plumbing that uses them lands separately.

use jetstream_wireformat::{Data, JetStreamWireFormat};

/// r[impl jetstream.subscription.compat]
/// The terminator, and cancellation, take **global** message ids in the
/// space below `MESSAGE_ID_START`, exactly as `RJETSTREAMERROR` already
/// does. That is what keeps `102 + 2 * index` intact: a streaming method
/// costs no extra per-method id, so `r[jetstream.rpc.swift.message-ids]`
/// and `r[jetstream.rpc.ts.message-ids]` do not change and no protocol is
/// re-generated for its unary methods.
///
/// 5, 6 and 7 are taken by the error frames; 100 and 101 by version. 106
/// and 107 are `TERROR`/`RERROR`, which overlap the generated ids for
/// method index 2 — latent today, since nothing decodes an `ErrorFrame`
/// on a service lane, but a reason to allocate here rather than there.
pub const RDONE: u8 = 8;

/// r[impl jetstream.subscription.cancel]
pub const TCANCEL: u8 = 9;
/// r[impl jetstream.subscription.cancel]
pub const RCANCEL: u8 = TCANCEL + 1;

/// r[impl jetstream.subscription.cancel]
/// Cancellation bears a **fresh** tag and names its target in the
/// payload, which is the shape 9P's `Tflush` uses and the one this
/// specification adopted after an earlier draft got the mechanics
/// backwards. Sending it under the subscription's own tag would put two
/// calls under one correlation key while that tag is still in flight.
#[derive(Debug, Clone, PartialEq, Eq, JetStreamWireFormat)]
pub struct Tcancel {
    /// The tag of the subscription being cancelled.
    pub oldtag: u16,
    /// r[impl jetstream.subscription.identity]
    /// Zero when the cancellation travels on the subscription's own lane,
    /// where `oldtag` is unambiguous. Otherwise the binding identifier,
    /// because tags are allocated per lane and a concurrent subscription
    /// elsewhere on the session may hold the same number.
    pub binding: u64,
}

/// r[impl jetstream.subscription.cancel]
/// The acknowledgement, which `r[jetstream.subscription.cancel]` requires
/// to arrive on the subscription's own lane after every item already
/// emitted there — that ordering is what makes the tag safe to reuse.
#[derive(Debug, Clone, PartialEq, Eq, JetStreamWireFormat)]
pub struct Rcancel {
    /// The tag that has now stopped emitting.
    pub oldtag: u16,
}

/// r[impl jetstream.lane.addressing]
/// The endpoint a subscription addresses within a peer — the room, the
/// object, the cell. An opaque byte string: codegen emits the client that
/// carries it and cannot know an application's naming, so any structure
/// is the application's to impose and no implementation's to interpret.
#[derive(Debug, Clone, PartialEq, Eq, JetStreamWireFormat)]
pub struct Endpoint(pub Data);

impl Endpoint {
    /// The endpoint naming nothing in particular: a peer that hosts one
    /// thing, which is every protocol written before this existed.
    pub fn root() -> Self {
        Endpoint(Data(Vec::new()))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0 .0
    }
}

impl From<&str> for Endpoint {
    fn from(name: &str) -> Self {
        Endpoint(Data(name.as_bytes().to_vec()))
    }
}

#[cfg(test)]
mod tests;

//! Session capabilities.
//!
//! r[impl jetstream.session.capabilities]
//! Capabilities are reported by the session, not inferred from the
//! concrete transport type and not reduced to a lowest common
//! denominator, so that a caller can pick a multiplexing strategy
//! without knowing which transport it is holding.

use std::fmt::{self, Display};

use crate::session::error::SessionError;

/// How many lanes a session can carry.
///
/// r[impl jetstream.session.single-lane]
/// A byte-stream transport reports [`LaneSupport::One`]: its first lane
/// open succeeds and every later one fails with
/// [`SessionError::LaneLimitReached`] rather than blocking or silently
/// sharing the lane that is already open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LaneSupport {
    /// Exactly one lane for the lifetime of the session.
    One,
    /// Any number of independent lanes.
    Many,
}

impl LaneSupport {
    /// Whether more than one lane can be open at a time.
    pub fn is_many(&self) -> bool {
        matches!(self, LaneSupport::Many)
    }
}

impl Display for LaneSupport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LaneSupport::One => write!(f, "one"),
            LaneSupport::Many => write!(f, "many"),
        }
    }
}

/// The peer identity model a session's transport establishes.
///
/// r[impl jetstream.session.identity]
/// The identity *value* travels in [`crate::context::Context`]; this is
/// the model, which is what a caller has to branch on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdentityKind {
    /// No transport-level peer authentication: unix sockets, plain TCP,
    /// in-process.
    None,
    /// A TLS peer certificate: QUIC and WebTransport.
    Certificate,
    /// A public key established by the transport itself: iroh.
    Key,
}

impl IdentityKind {
    /// r[impl jetstream.session.identity.addressing]
    /// Whether a caller must supply a network address to reach a peer.
    /// Under [`IdentityKind::Key`] the identity is the address, so a
    /// caller MUST NOT be required to carry placement information
    /// alongside it.
    pub fn requires_address(&self) -> bool {
        !matches!(self, IdentityKind::Key)
    }
}

impl Display for IdentityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdentityKind::None => write!(f, "none"),
            IdentityKind::Certificate => write!(f, "certificate"),
            IdentityKind::Key => write!(f, "key"),
        }
    }
}

/// A single capability, named so that a session can say what it is
/// missing when code asks for something it does not have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    /// More than one lane per session.
    ManyLanes,
    /// An unordered, unreliable datagram channel.
    Datagrams,
    /// Survival across a change of network path.
    Migration,
    /// Dialling a peer by identity alone, with no address.
    KeyAddressing,
}

impl Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Capability::ManyLanes => write!(f, "many lanes"),
            Capability::Datagrams => write!(f, "datagrams"),
            Capability::Migration => write!(f, "migration"),
            Capability::KeyAddressing => write!(f, "key addressing"),
        }
    }
}

/// What a session can do.
///
/// The constructors mirror the conformance table in the sessions and
/// lanes specification, so that a transport declares its row rather than
/// assembling one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Capabilities {
    /// Whether the session can carry more than one lane.
    pub lanes: LaneSupport,
    /// Whether the session has an unordered datagram channel.
    pub datagrams: bool,
    /// The peer identity model.
    pub identity: IdentityKind,
    /// Whether the session survives a change of network path.
    pub migration: bool,
}

impl Capabilities {
    /// r[impl jetstream.session.conformance.iroh]
    /// iroh: many lanes, datagrams, key identity, migrates.
    pub const fn iroh() -> Self {
        Self {
            lanes: LaneSupport::Many,
            datagrams: true,
            identity: IdentityKind::Key,
            migration: true,
        }
    }

    /// QUIC: many lanes, datagrams, certificate identity, migrates.
    pub const fn quic() -> Self {
        Self {
            lanes: LaneSupport::Many,
            datagrams: true,
            identity: IdentityKind::Certificate,
            migration: true,
        }
    }

    /// r[impl jetstream.session.conformance.webtransport]
    /// WebTransport over HTTP/3: many lanes, datagrams, certificate
    /// identity, migrates.
    pub const fn webtransport() -> Self {
        Self {
            lanes: LaneSupport::Many,
            datagrams: true,
            identity: IdentityKind::Certificate,
            migration: true,
        }
    }

    /// r[impl jetstream.session.conformance.single-stream]
    /// TCP, TLS, unix sockets and stdio: one lane, nothing else.
    pub const fn byte_stream() -> Self {
        Self {
            lanes: LaneSupport::One,
            datagrams: false,
            identity: IdentityKind::None,
            migration: false,
        }
    }

    /// r[impl jetstream.session.local]
    /// In-process: many lanes, no datagrams, no identity. Migration is
    /// not meaningful, so it is reported as absent.
    pub const fn in_process() -> Self {
        Self {
            lanes: LaneSupport::Many,
            datagrams: false,
            identity: IdentityKind::None,
            migration: false,
        }
    }

    /// Whether this session has `capability`.
    pub fn supports(&self, capability: Capability) -> bool {
        match capability {
            Capability::ManyLanes => self.lanes.is_many(),
            Capability::Datagrams => self.datagrams,
            Capability::Migration => self.migration,
            Capability::KeyAddressing => !self.identity.requires_address(),
        }
    }

    /// r[impl jetstream.session.capabilities.degradation]
    /// Fail explicitly for a capability the session does not have, so
    /// that the alternative — emulating it silently — is never the easy
    /// path.
    pub fn require(&self, capability: Capability) -> Result<(), SessionError> {
        if self.supports(capability) {
            Ok(())
        } else {
            Err(SessionError::Unsupported(capability))
        }
    }
}

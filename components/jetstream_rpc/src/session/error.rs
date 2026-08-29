//! Errors a session can report.

use crate::{session::capabilities::Capability, Error};

/// Something a session was asked to do and could not.
///
/// Every variant carries a stable code, reachable both from
/// [`SessionError::code`] and from [`Error::code`] once converted, so
/// that callers can inspect the reason instead of matching on a message.
#[derive(Debug, Clone, thiserror::Error)]
pub enum SessionError {
    /// r[impl jetstream.session.single-lane]
    /// The session carries one lane and that lane is already open.
    #[error("session supports one lane and it is already open")]
    LaneLimitReached,

    /// The session cannot accept peer-opened lanes. A client on a
    /// byte-stream transport is the usual case.
    #[error("session does not accept peer-opened lanes")]
    AcceptUnsupported,

    /// The session cannot open lanes. A server holding one accepted
    /// byte stream is the usual case.
    #[error("session does not open lanes")]
    OpenUnsupported,

    /// r[impl jetstream.session.lifetime]
    /// The session is closed. Opening, accepting and any in-flight call
    /// on one of its lanes fails with this rather than hanging.
    #[error("session is closed")]
    Closed,

    /// The lane is closed while its session is not: the peer end was
    /// dropped.
    #[error("lane is closed")]
    LaneClosed,

    /// r[impl jetstream.session.capabilities.degradation]
    /// Code asked for a capability this session does not report.
    #[error("session does not support {0}")]
    Unsupported(Capability),

    /// r[impl jetstream.session.datagrams]
    /// A datagram larger than the path limit. Rejected at the send site
    /// rather than fragmented.
    #[error("datagram of {size} bytes exceeds the {limit} byte limit")]
    DatagramTooLarge {
        /// Encoded size of the frame that was offered.
        size: u32,
        /// The path's datagram limit.
        limit: u32,
    },

    /// The underlying transport failed.
    #[error("transport error: {0}")]
    Transport(#[source] Error),
}

impl SessionError {
    /// The stable code for this error.
    pub fn code(&self) -> &'static str {
        match self {
            SessionError::LaneLimitReached => {
                "jetstream::session::lane_limit_reached"
            }
            SessionError::AcceptUnsupported => {
                "jetstream::session::accept_unsupported"
            }
            SessionError::OpenUnsupported => {
                "jetstream::session::open_unsupported"
            }
            SessionError::Closed => "jetstream::session::closed",
            SessionError::LaneClosed => "jetstream::session::lane_closed",
            SessionError::Unsupported(_) => "jetstream::session::unsupported",
            SessionError::DatagramTooLarge { .. } => {
                "jetstream::session::datagram_too_large"
            }
            SessionError::Transport(_) => "jetstream::session::transport",
        }
    }
}

impl From<SessionError> for Error {
    fn from(err: SessionError) -> Self {
        match err {
            SessionError::Transport(inner) => inner,
            other => {
                let code = other.code();
                Error::with_code(other.to_string(), code)
            }
        }
    }
}

impl From<Error> for SessionError {
    fn from(err: Error) -> Self {
        SessionError::Transport(err)
    }
}

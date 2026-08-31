//! Errors a session can report.

use crate::{session::capabilities::Capability, Error};

/// Something a session was asked to do and could not.
///
/// Every variant carries a stable code, reachable both from
/// [`SessionError::code`] and from [`Error::code`] once converted, so
/// that callers can inspect the reason instead of matching on a message.
///
/// [`SessionError::Transport`] is the one variant whose converted code
/// may differ: it carries an error the transport produced, and that
/// error's own code is more specific than a session-level one, so it is
/// kept when present. The session code is stamped only when the inner
/// error carries none, which is why a converted transport error always
/// has *a* code but not always this one.
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
    /// The code stamped on a transport error that carries none of its
    /// own.
    pub const TRANSPORT_CODE: &'static str = "jetstream::session::transport";

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
            SessionError::Transport(_) => Self::TRANSPORT_CODE,
        }
    }
}

impl From<SessionError> for Error {
    fn from(err: SessionError) -> Self {
        match err {
            // Keep the transport's own code, span trace and backtrace:
            // "the QUIC connection was reset" tells a caller more than
            // "a session transport failed". Stamp the session code only
            // when there is nothing to lose by it, so that a converted
            // transport error is never left uninspectable.
            SessionError::Transport(inner) => {
                if inner.code().is_some() {
                    inner
                } else {
                    inner.set_code(SessionError::TRANSPORT_CODE)
                }
            }
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

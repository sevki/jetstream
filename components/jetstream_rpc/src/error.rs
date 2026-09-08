use jetstream_wireformat::{JetStreamWireFormat, WireFormat};

use crate::Framer;

pub type Error = jetstream_error::Error;

pub(crate) const TLERROR: u8 = 6;

pub const RLERROR: u8 = TLERROR + 1;

pub const RJETSTREAMERROR: u8 = TLERROR - 1;

/// r[impl jetstream.error.v2.wireformat.error-frame]
/// A **global** id, below `MESSAGE_ID_START`, like every other id in this
/// frame.
///
/// It used to be 107 — 9P2000's `Rerror`, with a `TERROR = 106` beside it
/// that named a request 9P never had and that nothing here ever emitted.
/// `#[service]` allocates per-method ids as `MESSAGE_ID_START + 2 * index`
/// from 102, so 106 and 107 *are* the request and response of method
/// index 2: every protocol with three or more methods had this frame
/// sitting in the middle of its own range.
///
/// Harmless only because one code path did not exist. Nothing constructs
/// `ErrorFrame::RError`, so no encoder has ever written 107 — which is
/// also why moving it costs nothing: there is no peer speaking the old
/// id to break compatibility with. Its siblings were already down here
/// (`RJETSTREAMERROR` 5, `RLERROR` 7), so this is the outlier rejoining
/// them rather than a new convention.
pub const RERROR: u8 = 11;

#[derive(Debug, JetStreamWireFormat)]
pub struct Rlerror {
    pub ecode: u32,
}

// Rerror
#[derive(Debug, JetStreamWireFormat)]
pub struct Rerror {
    pub ename: String,
}

// r[impl jetstream.error.v2.wireformat.error-frame]
pub enum ErrorFrame {
    JetStreamError(Error),
    RlError(Rlerror),
    RError(Rerror),
}

impl Framer for ErrorFrame {
    fn message_type(&self) -> u8 {
        match self {
            ErrorFrame::JetStreamError(_) => RJETSTREAMERROR,
            ErrorFrame::RlError(_) => RLERROR,
            ErrorFrame::RError(_) => RERROR,
        }
    }

    fn byte_size(&self) -> u32 {
        match self {
            ErrorFrame::JetStreamError(error) => error.byte_size(),
            ErrorFrame::RlError(rlerror) => rlerror.byte_size(),
            ErrorFrame::RError(rerror) => rerror.byte_size(),
        }
    }

    fn encode<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match self {
            ErrorFrame::JetStreamError(error) => error.encode(writer),
            ErrorFrame::RlError(rlerror) => rlerror.encode(writer),
            ErrorFrame::RError(rerror) => rerror.encode(writer),
        }
    }

    fn decode<R: std::io::Read>(
        reader: &mut R,
        ty: u8,
    ) -> std::io::Result<Self> {
        match ty {
            RERROR => {
                let err = Rerror::decode(reader)?;
                Ok(ErrorFrame::RError(err))
            }
            RLERROR => {
                let err = Rlerror::decode(reader)?;
                Ok(ErrorFrame::RlError(err))
            }
            RJETSTREAMERROR => {
                let err = Error::decode(reader)?;
                Ok(ErrorFrame::JetStreamError(err))
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unknown message type: {}", ty),
            )),
        }
    }
}

impl From<Error> for ErrorFrame {
    fn from(err: Error) -> Self {
        ErrorFrame::JetStreamError(err)
    }
}

pub fn error_to_rmessage(err: &Rlerror) -> Error {
    let kind = match err.ecode as i32 {
        jetstream_libc::ENOENT => std::io::ErrorKind::NotFound,
        jetstream_libc::EPERM => std::io::ErrorKind::PermissionDenied,
        jetstream_libc::ECONNREFUSED => std::io::ErrorKind::ConnectionRefused,
        jetstream_libc::ECONNRESET => std::io::ErrorKind::ConnectionReset,
        jetstream_libc::ECONNABORTED => std::io::ErrorKind::ConnectionAborted,
        jetstream_libc::ENOTCONN => std::io::ErrorKind::NotConnected,
        jetstream_libc::EADDRINUSE => std::io::ErrorKind::AddrInUse,
        jetstream_libc::EADDRNOTAVAIL => std::io::ErrorKind::AddrNotAvailable,
        jetstream_libc::EPIPE => std::io::ErrorKind::BrokenPipe,
        jetstream_libc::EEXIST => std::io::ErrorKind::AlreadyExists,
        jetstream_libc::EWOULDBLOCK => std::io::ErrorKind::WouldBlock,
        jetstream_libc::EINVAL => std::io::ErrorKind::InvalidInput,
        jetstream_libc::ETIMEDOUT => std::io::ErrorKind::TimedOut,
        jetstream_libc::EINTR => std::io::ErrorKind::Interrupted,
        _ => std::io::ErrorKind::Other,
    };
    std::io::Error::new(kind, "rlerror".to_string()).into()
}

impl From<ErrorFrame> for Error {
    fn from(frame: ErrorFrame) -> Self {
        match frame {
            ErrorFrame::JetStreamError(error) => error,
            ErrorFrame::RlError(rlerror) => error_to_rmessage(&rlerror),
            ErrorFrame::RError(rerror) => Error::new(rerror.ename),
        }
    }
}

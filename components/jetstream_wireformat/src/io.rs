//! The I/O traits [`WireFormat`](crate::WireFormat) is written against.
//!
//! With the `std` feature these are just `std::io`. Without it, a minimal
//! allocation free replacement is provided: [`Read`] for `&[u8]` and
//! [`Write`] for `&mut [u8]`, so the caller injects every buffer.

#[cfg(feature = "std")]
pub use std::io::{Error, ErrorKind, Read, Result, Write};

#[cfg(not(feature = "std"))]
pub use no_std_io::*;

#[cfg(not(feature = "std"))]
mod no_std_io {
    use core::{fmt, mem};

    pub type Result<T> = core::result::Result<T, Error>;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[non_exhaustive]
    pub enum ErrorKind {
        InvalidInput,
        InvalidData,
        UnexpectedEof,
        WriteZero,
        Other,
    }

    /// Like `std::io::Error`, but the message is a `&'static str`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Error {
        kind: ErrorKind,
        msg: &'static str,
    }

    impl Error {
        pub fn new(kind: ErrorKind, msg: &'static str) -> Self {
            Error { kind, msg }
        }

        pub fn kind(&self) -> ErrorKind {
            self.kind
        }
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.msg)
        }
    }

    impl core::error::Error for Error {}

    pub trait Read {
        fn read_exact(&mut self, buf: &mut [u8]) -> Result<()>;
    }

    pub trait Write {
        fn write_all(&mut self, buf: &[u8]) -> Result<()>;
    }

    impl<R: Read + ?Sized> Read for &mut R {
        fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
            (**self).read_exact(buf)
        }
    }

    impl<W: Write + ?Sized> Write for &mut W {
        fn write_all(&mut self, buf: &[u8]) -> Result<()> {
            (**self).write_all(buf)
        }
    }

    impl Read for &[u8] {
        fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
            match self.split_at_checked(buf.len()) {
                Some((head, tail)) => {
                    buf.copy_from_slice(head);
                    *self = tail;
                    Ok(())
                }
                None => {
                    *self = &[];
                    Err(Error::new(
                        ErrorKind::UnexpectedEof,
                        "failed to fill whole buffer",
                    ))
                }
            }
        }
    }

    impl Write for &mut [u8] {
        fn write_all(&mut self, buf: &[u8]) -> Result<()> {
            if buf.len() > self.len() {
                return Err(Error::new(
                    ErrorKind::WriteZero,
                    "failed to write whole buffer",
                ));
            }
            let (head, tail) = mem::take(self).split_at_mut(buf.len());
            head.copy_from_slice(buf);
            *self = tail;
            Ok(())
        }
    }
}

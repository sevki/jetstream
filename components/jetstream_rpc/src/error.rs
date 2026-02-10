use jetstream_wireformat::WireFormat;

use crate::Framer;

pub type Error = jetstream_error::Error;

pub(crate) const TLERROR: u8 = 6;

pub const RLERROR: u8 = TLERROR + 1;

pub const RJETSTREAMERROR: u8 = TLERROR - 1;

// r[impl jetstream.error.v2.wireformat.error-frame]
pub struct ErrorFrame(Error);

impl Framer for ErrorFrame {
    fn message_type(&self) -> u8 {
        RJETSTREAMERROR
    }

    fn byte_size(&self) -> u32 {
        self.0.byte_size()
    }

    fn encode<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.encode(writer)
    }

    fn decode<R: std::io::Read>(
        reader: &mut R,
        ty: u8,
    ) -> std::io::Result<Self> {
        match ty {
            RJETSTREAMERROR => {
                let err = Error::decode(reader)?;
                Ok(ErrorFrame(err))
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
        ErrorFrame(err)
    }
}

impl From<ErrorFrame> for Error {
    fn from(frame: ErrorFrame) -> Self {
        frame.0
    }
}

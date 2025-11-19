use jetstream_wireformat::WireFormat;
use std::io;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Write;
use std::mem;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Frame<T: Framer> {
    pub tag: u16,
    pub msg: T,
}

impl<T: Framer> From<(u16, T)> for Frame<T> {
    fn from((tag, msg): (u16, T)) -> Self {
        Self { tag, msg }
    }
}

impl<T: Framer> WireFormat for Frame<T> {
    fn byte_size(&self) -> u32 {
        let msg_size = self.msg.byte_size();
        // size + type + tag + message size
        (mem::size_of::<u32>() + mem::size_of::<u8>() + mem::size_of::<u16>())
            as u32
            + msg_size
    }

    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.byte_size().encode(writer)?;

        let ty = self.msg.message_type();

        ty.encode(writer)?;
        self.tag.encode(writer)?;

        self.msg.encode(writer)?;

        Ok(())
    }

    fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
        let byte_size: u32 = WireFormat::decode(reader)?;

        // byte_size includes the size of byte_size so remove that from the
        // expected length of the message.  Also make sure that byte_size is at least
        // that long to begin with.
        if byte_size < mem::size_of::<u32>() as u32 {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("byte_size(= {byte_size}) is less than 4 bytes"),
            ));
        }
        let reader =
            &mut reader.take((byte_size - mem::size_of::<u32>() as u32) as u64);

        let mut ty = [0u8];
        reader.read_exact(&mut ty)?;

        let tag: u16 = WireFormat::decode(reader)?;
        let msg = T::decode(reader, ty[0])?;

        Ok(Frame { tag, msg })
    }
}

pub trait Framer: Sized + Send + Sync {
    fn message_type(&self) -> u8;
    /// Returns the number of bytes necessary to fully encode `self`.
    fn byte_size(&self) -> u32;

    /// Encodes `self` into `writer`.
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()>;

    /// Decodes `Self` from `reader`.
    fn decode<R: Read>(reader: &mut R, ty: u8) -> io::Result<Self>;
}

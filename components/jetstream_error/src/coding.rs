use jetstream_wireformat::WireFormat;
use tracing::Level;

// r[impl jetstream.error.v2.backtrace.level-codec]
pub(crate) struct LevelCodec;

impl LevelCodec {
    pub(crate) fn byte_size(_: &Level) -> u32 {
        1
    }

    pub(crate) fn encode<W: std::io::Write>(
        level: &Level,
        writer: &mut W,
    ) -> std::io::Result<()>
    where
        Self: Sized,
    {
        match *level {
            Level::TRACE => 0u8.encode(writer),
            Level::DEBUG => 1u8.encode(writer),
            Level::INFO => 2u8.encode(writer),
            Level::WARN => 3u8.encode(writer),
            Level::ERROR => 4u8.encode(writer),
        }
    }

    pub(crate) fn decode<R: std::io::Read>(
        reader: &mut R,
    ) -> std::io::Result<Level>
    where
        Self: Sized,
    {
        let level_byte = u8::decode(reader)?;
        match level_byte {
            0 => Ok(Level::TRACE),
            1 => Ok(Level::DEBUG),
            2 => Ok(Level::INFO),
            3 => Ok(Level::WARN),
            4 => Ok(Level::ERROR),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid level byte: {}", level_byte),
            )),
        }
    }
}

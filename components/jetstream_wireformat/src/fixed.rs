//! Fixed-size wire format collections, built on generic const args.
//!
//! Requires the `nightly` feature, a nightly compiler and `-Znext-solver`
//! (e.g. `RUSTFLAGS="-Znext-solver"`), because it relies on the unstable
//! `gca_const_items` feature. See
//! <https://blog.rust-lang.org/inside-rust/2026/10/02/generic-const-args-and-you/>.
//!
//! Unlike `Vec<T>`, a `[T; N]` carries no length prefix on the wire: the
//! length is part of the type, so the encoding is exactly `N` consecutive
//! elements.

use std::{
    gca,
    io::{self, Read, Write},
};

use crate::WireFormat;

/// A [`WireFormat`] type whose encoded size is known at compile time.
pub trait FixedWireFormat: WireFormat {
    /// Number of bytes every value of this type occupies on the wire.
    const WIRE_SIZE: usize;
}

macro_rules! fixed_impl {
    ($($Ty:ty => $size:expr),* $(,)?) => {
        $(impl FixedWireFormat for $Ty {
            const WIRE_SIZE: usize = $size;
        })*
    };
}

fixed_impl! {
    u8 => 1, u16 => 2, u32 => 4, u64 => 8, u128 => 16,
    i16 => 2, i32 => 4, i64 => 8, i128 => 16,
    f32 => 4, f64 => 8,
    usize => std::mem::size_of::<usize>(),
    isize => std::mem::size_of::<isize>(),
    bool => 1,
    () => 0,
}

impl<T: FixedWireFormat, const N: usize> FixedWireFormat for [T; N] {
    const WIRE_SIZE: usize = T::WIRE_SIZE * N;
}

impl<T: WireFormat, const N: usize> WireFormat for [T; N] {
    fn byte_size(&self) -> u32 {
        self.iter().map(WireFormat::byte_size).sum()
    }

    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.iter().try_for_each(|elem| elem.encode(writer))
    }

    fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
        let elems = (0..N)
            .map(|_| T::decode(reader))
            .collect::<io::Result<Vec<T>>>()?;
        elems
            .try_into()
            .map_err(|_| io::Error::other("array length mismatch"))
    }
}

/// Encode a [`FixedWireFormat`] value into a stack array sized by its type.
pub trait EncodeFixed: FixedWireFormat + Sized {
    /// Encodes `self` into an array of exactly [`FixedWireFormat::WIRE_SIZE`] bytes.
    fn encode_array(&self) -> io::Result<[u8; gca!(Self::WIRE_SIZE)]> {
        let mut buf = [0u8; _];
        let mut cursor = &mut buf[..];
        self.encode(&mut cursor)?;
        Ok(buf)
    }
}

impl<T: FixedWireFormat> EncodeFixed for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn array_roundtrip_has_no_length_prefix() {
        let v: [u32; 3] = [1, 2, 3];
        let mut bytes = Vec::new();
        v.encode(&mut bytes).unwrap();
        assert_eq!(bytes.len(), 12);
        assert_eq!(v.byte_size(), 12);
        assert_eq!(<[u32; 3]>::decode(&mut &bytes[..]).unwrap(), v);
    }

    #[test]
    fn encode_array_is_sized_by_type() {
        let a: [u8; 4] = 0x0102_0304u32.encode_array().unwrap();
        assert_eq!(a, [4, 3, 2, 1]);
        let nested: [u8; 8] = [1u16, 2, 3, 4].encode_array().unwrap();
        assert_eq!(nested, [1, 0, 2, 0, 3, 0, 4, 0]);
    }

    #[test]
    fn short_input_errors() {
        assert!(<[u16; 2]>::decode(&mut &[0u8, 1, 2][..]).is_err());
    }
}

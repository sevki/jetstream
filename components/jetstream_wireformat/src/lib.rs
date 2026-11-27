// Copyright (c) 2024, Sevki <s@sevki.io>
// Copyright 2018 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![cfg_attr(docsrs, feature(doc_cfg))]
//! 9P wire format encoding and decoding.
//!
//! This crate follows [Hard Mode Rust]: it is split into a `#![no_std]`,
//! allocation free core and a `std` shell.
//!
//! * Without the `std` feature the crate is `#![no_std]` and never allocates.
//!   The core defines [`WireFormat`], the [`io`] traits it is written against
//!   (reading from `&[u8]`, writing to `&mut [u8]`), and implementations for
//!   integers, floats, `bool`, `Option`, tuples and [`Wrapped`]. All
//!   resources (buffers) are injected by the caller.
//! * With `std` (default) `io` is `std::io`, and the types that need an
//!   allocator or an OS (`String`, `Vec`, collections, [`Data`], [`P9String`],
//!   `Url`, async/tokio support) are available too.
//!
//! [Hard Mode Rust]: https://matklad.github.io/2022/10/06/hard-mode-rust.html
#![cfg_attr(not(feature = "std"), no_std)]

use core::{marker::PhantomData, mem};

use zerocopy::LittleEndian;

pub mod io;
pub use io::{Read, Write};
pub use jetstream_macros::JetStreamWireFormat;

#[cfg(feature = "std")]
mod std_impls;
#[cfg(feature = "std")]
pub use std_impls::*;

#[cfg(feature = "std")]
pub mod wire_format_extensions;

#[cfg(all(feature = "std", target_arch = "wasm32"))]
pub mod wasm;

/// A type that can be encoded on the wire using the 9P protocol.
#[cfg(not(target_arch = "wasm32"))]
pub trait WireFormat: Send {
    /// Returns the number of bytes necessary to fully encode `self`.
    fn byte_size(&self) -> u32;

    /// Encodes `self` into `writer`.
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()>
    where
        Self: Sized;

    /// Decodes `Self` from `reader`.
    fn decode<R: Read>(reader: &mut R) -> io::Result<Self>
    where
        Self: Sized;
}

/// A type that can be encoded on the wire using the 9P protocol.
/// WebAssembly doesn't fully support Send, so we don't require it.
#[cfg(target_arch = "wasm32")]
pub trait WireFormat: core::marker::Sized {
    /// Returns the number of bytes necessary to fully encode `self`.
    fn byte_size(&self) -> u32;

    /// Encodes `self` into `writer`.
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()>;

    /// Decodes `Self` from `reader`.
    fn decode<R: Read>(reader: &mut R) -> io::Result<Self>;
}


// This doesn't really _need_ to be a macro but unfortunately there is no trait bound to
// express "can be casted to another type", which means we can't write `T as u8` in a trait
// based implementation.  So instead we have this macro, which is implemented for all the
// stable unsigned types with the added benefit of not being implemented for the signed
// types which are not allowed by the protocol.
macro_rules! uint_wire_format_impl {
    ($Ty:ty) => {
        impl WireFormat for $Ty {
            fn byte_size(&self) -> u32 {
                mem::size_of::<$Ty>() as u32
            }

            fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
                writer.write_all(&self.to_le_bytes())
            }

            fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
                let mut buf = [0; mem::size_of::<$Ty>()];
                reader.read_exact(&mut buf)?;
                paste::expr! {
                    let num: zerocopy::[<$Ty:snake:upper>]<LittleEndian> =  zerocopy::byteorder::[<$Ty:snake:upper>]::from_bytes(buf);
                    Ok(num.get())
                }
            }
        }
    };
}
// unsigned integers
uint_wire_format_impl!(u16);
uint_wire_format_impl!(u32);
uint_wire_format_impl!(u64);
uint_wire_format_impl!(u128);
// signed integers
uint_wire_format_impl!(i16);
uint_wire_format_impl!(i32);
uint_wire_format_impl!(i64);
uint_wire_format_impl!(i128);

macro_rules! float_wire_format_impl {
    ($Ty:ty) => {
        impl WireFormat for $Ty {
            fn byte_size(&self) -> u32 {
                mem::size_of::<$Ty>() as u32
            }

            fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
                paste::expr! {
                    writer.write_all(&self.to_le_bytes())
                }
            }

            fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
                let mut buf = [0; mem::size_of::<$Ty>()];
                reader.read_exact(&mut buf)?;
                paste::expr! {
                    let num: zerocopy::[<$Ty:snake:upper>]<LittleEndian> =  zerocopy::byteorder::[<$Ty:snake:upper>]::from_bytes(buf);
                    Ok(num.get())
                }
            }
        }
    };
}

float_wire_format_impl!(f32);
float_wire_format_impl!(f64);

macro_rules! tuple_wire_format_impl {
    ($( $name:ident ),+) => {
        #[allow(non_snake_case)]
        impl<$( $name ),+> WireFormat for ( $( $name ),+ )
        where
            $( $name: WireFormat ),+
        {
            fn byte_size(&self) -> u32 {
                let ( $( $name ),+ ) = self;
                0 $( + $name.byte_size() )+
            }

            fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
                let ( $( $name ),+ ) = self;
                $( $name.encode(writer)?; )+
                Ok(())
            }

            fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
                Ok((
                    $( $name::decode(reader)? ),+
                ))
            }
        }
    };
}

// Single-element tuple needs a hand-written impl because the macro expands
// `(A)` (parenthesized value) instead of `(A,)` (1-tuple).
#[allow(non_snake_case)]
impl<A: WireFormat> WireFormat for (A,) {
    fn byte_size(&self) -> u32 {
        self.0.byte_size()
    }

    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.0.encode(writer)
    }

    fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
        Ok((A::decode(reader)?,))
    }
}

tuple_wire_format_impl!(A, B);
tuple_wire_format_impl!(A, B, C);
tuple_wire_format_impl!(A, B, C, D);
tuple_wire_format_impl!(A, B, C, D, E);
tuple_wire_format_impl!(A, B, C, D, E, F);
tuple_wire_format_impl!(A, B, C, D, E, F, G);
tuple_wire_format_impl!(A, B, C, D, E, F, G, H);
tuple_wire_format_impl!(A, B, C, D, E, F, G, H, I);
tuple_wire_format_impl!(A, B, C, D, E, F, G, H, I, J);
tuple_wire_format_impl!(A, B, C, D, E, F, G, H, I, J, K);
tuple_wire_format_impl!(A, B, C, D, E, F, G, H, I, J, K, L);

impl WireFormat for u8 {
    fn byte_size(&self) -> u32 {
        1
    }

    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&[*self])
    }

    fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut byte = [0u8; 1];
        reader.read_exact(&mut byte)?;
        Ok(byte[0])
    }
}

impl WireFormat for usize {
    fn byte_size(&self) -> u32 {
        mem::size_of::<usize>() as u32
    }

    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.to_le_bytes())
    }

    fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut buf = [0; mem::size_of::<usize>()];
        reader.read_exact(&mut buf)?;
        Ok(usize::from_le_bytes(buf))
    }
}

impl WireFormat for isize {
    fn byte_size(&self) -> u32 {
        mem::size_of::<isize>() as u32
    }

    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.to_le_bytes())
    }

    fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut buf = [0; mem::size_of::<isize>()];
        reader.read_exact(&mut buf)?;
        Ok(isize::from_le_bytes(buf))
    }
}


impl<T> WireFormat for Option<T>
where
    T: WireFormat,
{
    fn byte_size(&self) -> u32 {
        1 + match self {
            None => 0,
            Some(value) => value.byte_size(),
        }
    }

    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        match self {
            None => WireFormat::encode(&0u8, writer),
            Some(value) => {
                WireFormat::encode(&1u8, writer)?;
                WireFormat::encode(value, writer)
            }
        }
    }

    fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
        let tag: u8 = WireFormat::decode(reader)?;
        match tag {
            0 => Ok(None),
            1 => Ok(Some(WireFormat::decode(reader)?)),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid Option tag",
            )),
        }
    }
}

impl WireFormat for () {
    fn byte_size(&self) -> u32 {
        0
    }

    fn encode<W: Write>(&self, _writer: &mut W) -> io::Result<()> {
        Ok(())
    }

    fn decode<R: Read>(_reader: &mut R) -> io::Result<Self> {
        Ok(())
    }
}

impl WireFormat for bool {
    fn byte_size(&self) -> u32 {
        1
    }

    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&[*self as u8])
    }

    fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut byte = [0u8; 1];
        reader.read_exact(&mut byte)?;
        match byte[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid byte for bool",
            )),
        }
    }
}


#[repr(transparent)]
pub struct Wrapped<T, I>(pub T, PhantomData<I>);

impl<T, I> Wrapped<T, I> {
    pub fn new(value: T) -> Self {
        Wrapped(value, PhantomData)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<T, I> WireFormat for Wrapped<T, I>
where
    T: Send + core::convert::AsRef<I>,
    I: WireFormat + core::convert::Into<T>,
{
    fn byte_size(&self) -> u32 {
        AsRef::<I>::as_ref(&self.0).byte_size()
    }

    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        AsRef::<I>::as_ref(&self.0).encode(writer)
    }

    fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
        let inner = I::decode(reader)?;
        Ok(Wrapped(inner.into(), PhantomData))
    }
}

#[cfg(target_arch = "wasm32")]
impl<T, I> WireFormat for Wrapped<T, I>
where
    T: core::convert::AsRef<I>,
    I: WireFormat + core::convert::Into<T>,
{
    fn byte_size(&self) -> u32 {
        AsRef::<I>::as_ref(&self.0).byte_size()
    }

    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        AsRef::<I>::as_ref(&self.0).encode(writer)
    }

    fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
        let inner = I::decode(reader)?;
        Ok(Wrapped(inner.into(), PhantomData))
    }
}


impl<T: WireFormat> WireFormat for PhantomData<T> {
    fn byte_size(&self) -> u32 {
        0
    }

    fn encode<W: io::Write>(&self, _writer: &mut W) -> io::Result<()>
    where
        Self: Sized,
    {
        Ok(())
    }

    fn decode<R: io::Read>(_reader: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        Ok(PhantomData)
    }
}


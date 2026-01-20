#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#[cfg(feature = "std")]
use std::collections::{BTreeMap, BinaryHeap};
// Copyright (c) 2024, Sevki <s@sevki.io>
// Copyright 2018 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.
use std::{
    collections::{BTreeSet, VecDeque},
    ffi::{CStr, CString, OsStr},
    fmt,
    io::{self, ErrorKind, Read, Write},
    marker::PhantomData,
    mem,
    ops::{Deref, DerefMut},
    string::String,
    vec::Vec,
};

use bytes::Buf;
pub use jetstream_macros::JetStreamWireFormat;
use zerocopy::LittleEndian;

pub mod miette;
pub mod wire_format_extensions;

#[cfg(target_arch = "wasm32")]
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
pub trait WireFormat: std::marker::Sized {
    /// Returns the number of bytes necessary to fully encode `self`.
    fn byte_size(&self) -> u32;

    /// Encodes `self` into `writer`.
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()>;

    /// Decodes `Self` from `reader`.
    fn decode<R: Read>(reader: &mut R) -> io::Result<Self>;
}

/// A 9P protocol string.
///
/// The string is always valid UTF-8 and 65535 bytes or less (enforced by `P9String::new()`).
///
/// It is represented as a C string with a terminating 0 (NUL) character to allow it to be passed
/// directly to libc functions.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct P9String {
    cstr: CString,
}

impl P9String {
    pub fn new(string_bytes: impl Into<Vec<u8>>) -> io::Result<Self> {
        let string_bytes: Vec<u8> = string_bytes.into();

        if string_bytes.len() > u16::MAX as usize {
            return Err(io::Error::new(
                ErrorKind::InvalidInput,
                "string is too long",
            ));
        }

        // 9p strings must be valid UTF-8.
        let _check_utf8 = std::str::from_utf8(&string_bytes)
            .map_err(|e| io::Error::new(ErrorKind::InvalidInput, e))?;

        let cstr = CString::new(string_bytes)
            .map_err(|e| io::Error::new(ErrorKind::InvalidInput, e))?;

        Ok(P9String { cstr })
    }

    pub fn len(&self) -> usize {
        self.cstr.as_bytes().len()
    }

    pub fn is_empty(&self) -> bool {
        self.cstr.as_bytes().is_empty()
    }

    pub fn as_c_str(&self) -> &CStr {
        self.cstr.as_c_str()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.cstr.as_bytes()
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// Returns a raw pointer to the string's storage.
    ///
    /// The string bytes are always followed by a NUL terminator ('\0'), so the pointer can be
    /// passed directly to libc functions that expect a C string.
    pub fn as_ptr(&self) -> *const libc::c_char {
        self.cstr.as_ptr()
    }

    #[cfg(target_arch = "wasm32")]
    /// Returns a raw pointer to the string's storage.
    ///
    /// The string bytes are always followed by a NUL terminator ('\0').
    /// Note: In WebAssembly, returns a raw pointer but libc is not available.
    pub fn as_ptr(&self) -> *const std::os::raw::c_char {
        self.cstr.as_ptr()
    }
}

impl PartialEq<&str> for P9String {
    fn eq(&self, other: &&str) -> bool {
        self.cstr.as_bytes() == other.as_bytes()
    }
}

impl TryFrom<&OsStr> for P9String {
    type Error = io::Error;

    fn try_from(value: &OsStr) -> io::Result<Self> {
        let string_bytes = value.as_encoded_bytes();
        Self::new(string_bytes)
    }
}

// The 9P protocol requires that strings are UTF-8 encoded.  The wire format is a u16
// count |N|, encoded in little endian, followed by |N| bytes of UTF-8 data.
impl WireFormat for P9String {
    fn byte_size(&self) -> u32 {
        (mem::size_of::<u16>() + self.len()) as u32
    }

    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        (self.len() as u16).encode(writer)?;
        writer.write_all(self.cstr.as_bytes())
    }

    fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
        let len: u16 = WireFormat::decode(reader)?;
        let mut string_bytes = vec![0u8; usize::from(len)];
        reader.read_exact(&mut string_bytes)?;
        Self::new(string_bytes)
    }
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

// The 9P protocol requires that strings are UTF-8 encoded.  The wire format is a u16
// count |N|, encoded in little endian, followed by |N| bytes of UTF-8 data.
impl WireFormat for String {
    fn byte_size(&self) -> u32 {
        (mem::size_of::<u16>() + self.len()) as u32
    }

    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        if self.len() > u16::MAX as usize {
            return Err(io::Error::new(
                ErrorKind::InvalidInput,
                "string is too long",
            ));
        }

        (self.len() as u16).encode(writer)?;
        writer.write_all(self.as_bytes())
    }

    fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
        let len: u16 = WireFormat::decode(reader)?;
        let mut result = String::with_capacity(len as usize);
        reader.take(len as u64).read_to_string(&mut result)?;
        Ok(result)
    }
}

// The wire format for repeated types is similar to that of strings: a little endian
// encoded u16 |N|, followed by |N| instances of the given type.
impl<T: WireFormat> WireFormat for Vec<T> {
    fn byte_size(&self) -> u32 {
        mem::size_of::<u16>() as u32
            + self.iter().map(|elem| elem.byte_size()).sum::<u32>()
    }

    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        if self.len() > u16::MAX as usize {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "too many elements in vector",
            ));
        }

        (self.len() as u16).encode(writer)?;
        for elem in self {
            elem.encode(writer)?;
        }

        Ok(())
    }

    fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
        let len: u16 = WireFormat::decode(reader)?;
        let mut result = Vec::with_capacity(len as usize);

        for _ in 0..len {
            result.push(WireFormat::decode(reader)?);
        }

        Ok(result)
    }
}

/// A type that encodes an arbitrary number of bytes of data.  Typically used for Rread
/// Twrite messages.  This differs from a `Vec<u8>` in that it encodes the number of bytes
/// using a `u32` instead of a `u16`.
#[derive(PartialEq, Eq, Clone)]
#[repr(transparent)]
#[cfg_attr(feature = "testing", derive(serde::Serialize, serde::Deserialize))]
pub struct Data(pub Vec<u8>);

// The maximum length of a data buffer that we support.  In practice the server's max message
// size should prevent us from reading too much data so this check is mainly to ensure a
// malicious client cannot trick us into allocating massive amounts of memory.
const MAX_DATA_LENGTH: u32 = 32 * 1024 * 1024;

impl fmt::Debug for Data {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // There may be a lot of data and we don't want to spew it all out in a trace.  Instead
        // just print out the number of bytes in the buffer.
        write!(f, "Data({} bytes)", self.len())
    }
}

// Implement Deref and DerefMut so that we don't have to use self.0 everywhere.
impl Deref for Data {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Data {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// Same as Vec<u8> except that it encodes the length as a u32 instead of a u16.
impl WireFormat for Data {
    fn byte_size(&self) -> u32 {
        mem::size_of::<u32>() as u32
            + self.iter().map(|elem| elem.byte_size()).sum::<u32>()
    }

    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        if self.len() > u32::MAX as usize {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "data is too large",
            ));
        }
        (self.len() as u32).encode(writer)?;
        writer.write_all(self)
    }

    fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
        let len: u32 = WireFormat::decode(reader)?;
        if len > MAX_DATA_LENGTH {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("data length ({} bytes) is too large", len),
            ));
        }

        let mut buf = Vec::with_capacity(len as usize);
        reader.take(len as u64).read_to_end(&mut buf)?;

        if buf.len() == len as usize {
            Ok(Data(buf))
        } else {
            Err(io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!(
                    "unexpected end of data: want: {} bytes, got: {} bytes",
                    len,
                    buf.len()
                ),
            ))
        }
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
                format!("Invalid Option tag: {}", tag),
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

impl io::Read for Data {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.reader().read(buf)
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
    T: Send + std::convert::AsRef<I>,
    I: WireFormat + std::convert::Into<T>,
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
    T: std::convert::AsRef<I>,
    I: WireFormat + std::convert::Into<T>,
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

impl<T: WireFormat> WireFormat for Box<T> {
    fn byte_size(&self) -> u32 {
        (**self).byte_size()
    }

    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()>
    where
        Self: Sized,
    {
        (**self).encode(writer)
    }

    fn decode<R: Read>(reader: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let inner = T::decode(reader)?;
        Ok(Box::new(inner))
    }
}

#[cfg(feature = "std")]
impl<K: WireFormat + Send + Sync + Ord, V: WireFormat + Send + Sync> WireFormat
    for BTreeMap<K, V>
{
    fn byte_size(&self) -> u32 {
        self.iter()
            .fold(0, |acc, (k, v)| acc + k.byte_size() + v.byte_size())
            + 2
    }

    fn encode<W: io::Write>(&self, writer: &mut W) -> io::Result<()>
    where
        Self: Sized,
    {
        if self.len() > u16::MAX as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Map too large",
            ));
        }
        let len = self.len() as u16;
        len.encode(writer)?;
        for (k, v) in self {
            k.encode(writer)?;
            v.encode(writer)?;
        }
        Ok(())
    }

    fn decode<R: io::Read>(reader: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let len: u16 = WireFormat::decode(reader)?;
        let mut map = BTreeMap::new();
        for _ in 0..len {
            let k = K::decode(reader)?;
            let v = V::decode(reader)?;
            map.insert(k, v);
        }
        Ok(map)
    }
}

#[cfg(feature = "std")]
impl<V: WireFormat + Send + Sync + Ord> WireFormat for BinaryHeap<V> {
    fn byte_size(&self) -> u32 {
        self.as_slice()
            .iter()
            .fold(0, |acc, elem| acc + elem.byte_size())
            + 2
    }

    fn encode<W: io::Write>(&self, writer: &mut W) -> io::Result<()>
    where
        Self: Sized,
    {
        if self.len() > u16::MAX as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Map too large",
            ));
        }
        let len = self.len() as u16;
        len.encode(writer)?;
        for elem in self {
            elem.encode(writer)?;
        }
        Ok(())
    }

    fn decode<R: io::Read>(reader: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let len: u16 = WireFormat::decode(reader)?;
        let mut heap = BinaryHeap::new();
        for _ in 0..len {
            let elem = V::decode(reader)?;
            heap.push(elem);
        }
        Ok(heap)
    }
}

impl<V: WireFormat + Send + Sync + Ord> WireFormat for VecDeque<V> {
    fn byte_size(&self) -> u32 {
        self.iter().fold(0, |acc, elem| acc + elem.byte_size()) + 2
    }

    fn encode<W: io::Write>(&self, writer: &mut W) -> io::Result<()>
    where
        Self: Sized,
    {
        if self.len() > u16::MAX as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Map too large",
            ));
        }
        let len = self.len() as u16;
        len.encode(writer)?;
        for elem in self {
            elem.encode(writer)?;
        }
        Ok(())
    }

    fn decode<R: io::Read>(reader: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let len: u16 = WireFormat::decode(reader)?;
        let mut deque = VecDeque::with_capacity(len as usize);
        for _ in 0..len {
            let elem = V::decode(reader)?;
            deque.push_back(elem);
        }
        Ok(deque)
    }
}

impl<V: WireFormat + Send + Sync + Ord> WireFormat for BTreeSet<V> {
    fn byte_size(&self) -> u32 {
        self.iter().fold(0, |acc, elem| acc + elem.byte_size()) + 2
    }

    fn encode<W: io::Write>(&self, writer: &mut W) -> io::Result<()>
    where
        Self: Sized,
    {
        if self.len() > u16::MAX as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Map too large",
            ));
        }
        let len = self.len() as u16;
        len.encode(writer)?;
        for elem in self {
            elem.encode(writer)?;
        }
        Ok(())
    }

    fn decode<R: io::Read>(reader: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let len: u16 = WireFormat::decode(reader)?;
        let mut set = BTreeSet::new();
        for _ in 0..len {
            let elem = V::decode(reader)?;
            set.insert(elem);
        }
        Ok(set)
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

impl WireFormat for url::Url {
    fn byte_size(&self) -> u32 {
        self.to_string().byte_size()
    }

    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()>
    where
        Self: Sized,
    {
        self.to_string().encode(writer)
    }

    fn decode<R: Read>(reader: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let string = String::decode(reader)?;
        url::Url::parse(&string)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

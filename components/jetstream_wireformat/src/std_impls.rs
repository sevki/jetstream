// Copyright (c) 2024, Sevki <s@sevki.io>
// Copyright 2018 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.
//! Everything that needs `std`: allocation, collections and the OS.
use std::{
    collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque},
    ffi::{CStr, CString, OsStr},
    fmt,
    hash::Hash,
    io::ErrorKind,
    mem,
    ops::{Deref, DerefMut},
};

use bytes::Buf;
use hashbrown::{HashMap, HashSet};

use crate::{
    io::{self, Read, Write},
    WireFormat,
};

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

impl io::Read for Data {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.reader().read(buf)
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

impl<T: WireFormat + Send + Sync + Eq + Hash> WireFormat for HashSet<T> {
    fn byte_size(&self) -> u32 {
        self.iter().fold(0, |acc, v| acc + v.byte_size()) + 2
    }

    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()>
    where
        Self: Sized,
    {
        if self.len() > u16::MAX as usize {
            return Err(io::Error::new(io::ErrorKind::Other, "Set too large"));
        }
        (self.len() as u16).encode(writer)?;
        for v in self.iter() {
            v.encode(writer)?;
        }
        Ok(())
    }

    fn decode<R: Read>(reader: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let len: u16 = WireFormat::decode(reader)?;
        let mut set = Self::with_capacity(len as usize);
        for _ in 0..len {
            let v = T::decode(reader)?;
            set.insert(v);
        }
        Ok(set)
    }
}

impl<
        K: WireFormat + Send + Sync + Ord + Eq + Hash,
        V: WireFormat + Send + Sync,
    > WireFormat for HashMap<K, V>
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
        (self.len() as u16).encode(writer)?;
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
        let mut map = HashMap::new();
        for _ in 0..len {
            let k = K::decode(reader)?;
            let v = V::decode(reader)?;
            map.insert(k, v);
        }
        Ok(map)
    }
}

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


// Copyright (c) 2024, Sevki <s@sevki.io>
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.
use core::net::{
    IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6,
};
use std::{
    io::{self},
    time::{Duration, SystemTime},
};

use bytes::Bytes;

pub extern crate bytes;

use super::WireFormat;

pub trait AsyncWireFormat: std::marker::Sized {
    fn encode_async<W: AsyncWireFormat + Unpin + Send>(
        self,
        writer: &mut W,
    ) -> impl std::future::Future<Output = io::Result<()>> + Send;
    fn decode_async<R: AsyncWireFormat + Unpin + Send>(
        reader: &mut R,
    ) -> impl std::future::Future<Output = io::Result<Self>> + Send;
}

#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
pub mod tokio {
    use std::{future::Future, io};

    use tokio::io::{AsyncRead, AsyncWrite};

    use crate::WireFormat;
    /// Extension trait for asynchronous wire format encoding and decoding.
    pub trait AsyncWireFormatExt
    where
        Self: WireFormat + Send,
    {
        /// Encodes the object asynchronously into the provided writer.
        ///
        /// # Arguments
        ///
        /// * `writer` - The writer to encode the object into.n
        ///
        /// # Returns
        ///
        /// A future that resolves to an `io::Result<()>` indicating the success or failure of the encoding operation.
        fn encode_async<W>(
            self,
            writer: W,
        ) -> impl Future<Output = io::Result<()>>
        where
            Self: Sync + Sized,
            W: AsyncWrite + Unpin + Send,
        {
            let mut writer = tokio_util::io::SyncIoBridge::new(writer);
            async {
                tokio::task::block_in_place(move || self.encode(&mut writer))
            }
        }

        /// Decodes an object asynchronously from the provided reader.
        ///
        /// # Arguments
        ///
        /// * `reader` - The reader to decode the object from.
        ///
        /// # Returns
        ///
        /// A future that resolves to an `io::Result<Self>` indicating the success or failure of the decoding operation.
        fn decode_async<R>(
            reader: R,
        ) -> impl Future<Output = io::Result<Self>> + Send
        where
            Self: Sync + Sized,
            R: AsyncRead + Unpin + Send,
        {
            let mut reader = tokio_util::io::SyncIoBridge::new(reader);
            async {
                tokio::task::block_in_place(move || Self::decode(&mut reader))
            }
        }
    }
    /// Implements the `AsyncWireFormatExt` trait for types that implement the `WireFormat` trait and can be sent across threads.
    impl<T: WireFormat + Send> AsyncWireFormatExt for T {}
}

/// A trait for converting types to and from a wire format.
pub trait ConvertWireFormat: WireFormat {
    /// Converts the type to a byte representation.
    ///
    /// # Returns
    ///
    /// A `Bytes` object representing the byte representation of the type.
    fn to_bytes(&self) -> Bytes;

    /// Converts a byte buffer to the type.
    ///
    /// # Arguments
    ///
    /// * `buf` - A mutable reference to a `Bytes` object containing the byte buffer.
    ///
    /// # Returns
    ///
    /// A `Result` containing the converted type or an `std::io::Error` if the conversion fails.
    fn from_bytes(buf: &Bytes) -> Result<Self, std::io::Error>
    where
        Self: Sized;

    /// AsRef<[u8]> for the type.
    ///
    /// # Returns
    ///
    /// A reference to the byte representation of the type.
    fn as_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

/// Implements the `ConvertWireFormat` trait for types that implement `jetstream_p9::WireFormat`.
/// This trait provides methods for converting the type to and from bytes.
impl<T> ConvertWireFormat for T
where
    T: WireFormat,
{
    /// Converts the type to bytes.
    /// Returns a `Bytes` object containing the encoded bytes.
    fn to_bytes(&self) -> Bytes {
        let mut buf = vec![];
        let res = self.encode(&mut buf);
        if let Err(e) = res {
            panic!("Failed to encode: {}", e);
        }
        Bytes::from(buf)
    }

    /// Converts bytes to the type.
    /// Returns a `Result` containing the decoded type or an `std::io::Error` if decoding fails.
    fn from_bytes(buf: &Bytes) -> Result<Self, std::io::Error> {
        let buf = buf.to_vec();
        T::decode(&mut buf.as_slice())
    }
}

impl WireFormat for Ipv4Addr {
    fn byte_size(&self) -> u32 {
        self.octets().len() as u32
    }

    fn encode<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.octets())
    }

    fn decode<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        Ok(Ipv4Addr::from(buf))
    }
}

impl WireFormat for Ipv6Addr {
    fn byte_size(&self) -> u32 {
        self.octets().len() as u32
    }

    fn encode<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.octets())
    }

    fn decode<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let mut buf = [0u8; 16];
        reader.read_exact(&mut buf)?;
        Ok(Ipv6Addr::from(buf))
    }
}

impl WireFormat for IpAddr {
    fn byte_size(&self) -> u32 {
        1 + match self {
            IpAddr::V4(ip) => ip.byte_size(),
            IpAddr::V6(ip) => ip.byte_size(),
        }
    }

    fn encode<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        match self {
            IpAddr::V4(ip) => {
                writer.write_all(&[4])?;
                ip.encode(writer)
            }
            IpAddr::V6(ip) => {
                writer.write_all(&[6])?;
                ip.encode(writer)
            }
        }
    }

    fn decode<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf)?;
        match buf[0] {
            4 => Ok(IpAddr::V4(Ipv4Addr::decode(reader)?)),
            6 => Ok(IpAddr::V6(Ipv6Addr::decode(reader)?)),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid IP address type",
            )),
        }
    }
}

impl WireFormat for SocketAddrV4 {
    fn byte_size(&self) -> u32 {
        self.ip().byte_size() + 2
    }

    fn encode<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        self.ip().encode(writer)?;
        self.port().encode(writer)
    }

    fn decode<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        self::Ipv4Addr::decode(reader).and_then(|ip| {
            u16::decode(reader).map(|port| SocketAddrV4::new(ip, port))
        })
    }
}

impl WireFormat for SocketAddrV6 {
    fn byte_size(&self) -> u32 {
        self.ip().byte_size() + 2
    }

    fn encode<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        self.ip().encode(writer)?;
        self.port().encode(writer)
    }

    fn decode<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        self::Ipv6Addr::decode(reader).and_then(|ip| {
            u16::decode(reader).map(|port| SocketAddrV6::new(ip, port, 0, 0))
        })
    }
}

impl WireFormat for SocketAddr {
    fn byte_size(&self) -> u32 {
        1 + match self {
            SocketAddr::V4(socket_addr_v4) => socket_addr_v4.byte_size(),
            SocketAddr::V6(socket_addr_v6) => socket_addr_v6.byte_size(),
        }
    }

    fn encode<W: io::Write>(&self, writer: &mut W) -> io::Result<()>
    where
        Self: Sized,
    {
        match self {
            SocketAddr::V4(socket_addr_v4) => {
                writer.write_all(&[4])?;
                socket_addr_v4.encode(writer)
            }
            SocketAddr::V6(socket_addr_v6) => {
                writer.write_all(&[6])?;
                socket_addr_v6.encode(writer)
            }
        }
    }

    fn decode<R: io::Read>(reader: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf)?;
        match buf[0] {
            4 => Ok(SocketAddr::V4(SocketAddrV4::decode(reader)?)),
            6 => Ok(SocketAddr::V6(SocketAddrV6::decode(reader)?)),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid address type",
            )),
        }
    }
}

/// Implement WireFormat for SystemTime, must be used with care as it can lead to overflow errors on Windows.
impl WireFormat for SystemTime {
    fn byte_size(&self) -> u32 {
        // u64 for millis since epoch
        8
    }

    fn encode<W: io::Write>(&self, writer: &mut W) -> io::Result<()>
    where
        Self: Sized,
    {
        let duration = self
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        (duration.as_millis() as u64).encode(writer)
    }

    fn decode<R: io::Read>(reader: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let millis = u64::decode(reader)?;
        SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_millis(millis))
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "timestamp overflow")
            })
    }
}

use std::{
    error::Error,
    pin::Pin,
};

use crate::coding::{Rframe, Tframe, WireFormat};
use bytes::{BufMut, Bytes, BytesMut};
use futures::prelude::*;

/// Message trait for JetStream messages, which need to implement the `WireFormat` trait.
pub trait Message: WireFormat + Send + Sync {}

/// A trait for implementing a 9P service.
/// This trait is implemented for types that can handle 9P requests.
pub trait JetStreamService<Req: Message, Resp: Message>:
    Send + Sync + Sized
{
    #[allow(clippy::type_complexity)]
    fn call(
        &mut self,
        req: Req,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Resp, Box<dyn Error + Send + Sync>>>
                + Send,
        >,
    >;
}

/// A trait for implementing a 9P service.
/// This trait is implemented for types that can handle 9P requests.
pub trait NinePService:
    JetStreamService<Tframe, Rframe> + Send + Sync + Clone + Clone
{
}

/// A service that implements the 9P protocol.
#[derive(Debug, Clone, Copy)]
pub struct NinePServiceImpl<S: NinePService> {
    inner: S,
}

impl<S: NinePService> NinePServiceImpl<S> {
    pub fn new(inner: S) -> Self {
        NinePServiceImpl { inner }
    }
}

impl<S: NinePService> JetStreamService<Tframe, Rframe> for NinePServiceImpl<S> {
    fn call(
        &mut self,
        req: Tframe,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Rframe, Box<dyn Error + Send + Sync>>>
                + Send,
        >,
    > {
        self.inner.call(req)
    }
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
    fn from_bytes(buf: &mut Bytes) -> Result<Self, std::io::Error>;
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
        let mut bytes = BytesMut::new();
        bytes.put_slice(buf.as_slice());
        bytes.freeze()
    }

    /// Converts bytes to the type.
    /// Returns a `Result` containing the decoded type or an `std::io::Error` if decoding fails.
    fn from_bytes(buf: &mut Bytes) -> Result<Self, std::io::Error> {
        let buf = buf.to_vec();
        T::decode(&mut buf.as_slice())
    }
}

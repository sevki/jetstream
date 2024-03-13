use std::{
    error::Error,
    io::{Read, Write},
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use async_wire_format::AsyncWireFormatExt;
use bytes::{BufMut, Bytes, BytesMut};
use futures::prelude::*;
use p9::{Rframe, Rmessage, Tframe, Tmessage, WireFormat};
pub use p9_wire_format_derive::P9WireFormat;
use tower::Service;

pub trait Message: WireFormat + Send + Sync {}

/// A trait for implementing a 9P service.
/// This trait is implemented for types that can handle 9P requests.
pub trait JetStreamService<Req: Message, Resp: Message>:
    Send + Sync + Sized
{
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

/// A static 9p service that always returns a version message.
#[derive(Debug, Clone, Copy)]
pub struct Radar;

#[derive(Debug, Clone, P9WireFormat)]
struct Ping(u8);

impl Message for Ping {}

#[derive(Debug, Clone, P9WireFormat)]
struct Pong(u8);

impl Message for Pong {}

impl JetStreamService<Ping, Pong> for Radar {
    fn call(
        &mut self,
        req: Ping,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Pong, Box<dyn Error + Send + Sync>>>
                + Send,
        >,
    > {
        Box::pin(async move { Ok(Pong(req.0)) })
    }
}

mod ninepecho {
    use super::*;

    #[derive(Debug, Clone, Copy)]
    pub struct EchoService;

    impl JetStreamService<Tframe, Rframe> for EchoService {
        fn call(
            &mut self,
            req: Tframe,
        ) -> Pin<
            Box<
                dyn Future<
                        Output = Result<Rframe, Box<dyn Error + Send + Sync>>,
                    > + Send,
            >,
        > {
            Box::pin(async move {
                Ok(Rframe {
                    tag: 0,
                    msg: Rmessage::Version(p9::Rversion {
                        msize: 0,
                        version: "9P2000".to_string(),
                    }),
                })
            })
        }
    }
}

struct Echo;

impl Service<bytes::Bytes> for Echo {
    type Error = Box<dyn Error + Send + Sync>;
    type Future = Pin<
        Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;
    type Response = bytes::Bytes;

    fn poll_ready(
        &mut self,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: bytes::Bytes) -> Self::Future {
        Box::pin(async move { Ok(req) })
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

impl<T: p9::WireFormat> ConvertWireFormat for T {
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

    fn from_bytes(buf: &mut Bytes) -> Result<Self, std::io::Error> {
        let buf = buf.to_vec();
        T::decode(&mut buf.as_slice())
    }
}

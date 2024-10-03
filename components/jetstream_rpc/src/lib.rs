use std::io;
use std::sync::Arc;
use std::error::Error;

use jetstream_wireformat::{
    wire_format_extensions::AsyncWireFormatExt, WireFormat,
};
use okstd::prelude::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Message trait for JetStream messages, which need to implement the `WireFormat` trait.
pub trait Message: WireFormat + Send + Sync {}

pub trait JetStreamProtocol {
    type Request: Message;
    type Response: Message;
}

pub trait JetStreamService: JetStreamProtocol + Send + Sync + Sized {
    fn rpc(
        &mut self,
        req: Self::Request,
    ) -> Result<Self::Response, Box<dyn Error + Send + Sync>>;

    fn handle_message<R, W>(
        &mut self,
        reader: &mut R,
        writer: &mut W,
    ) -> Result<(), io::Error>
    where
        R: std::io::Read,
        W: std::io::Write,
    {
        let req = Self::Request::decode(reader)?;
        let resp = self
            .rpc(req)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err));
        match resp {
            Ok(resp) => resp.encode(writer),
            Err(err) => Err(io::Error::new(io::ErrorKind::Other, err)),
        }
    }
}

/// A trait for implementing a 9P service.
/// This trait is implemented for types that can handle 9P requests.
#[async_trait::async_trait]
pub trait JetStreamAsyncService:
    JetStreamProtocol + Send + Sync + Sized
{
    async fn rpc(
        &mut self,
        req: Self::Request,
    ) -> Result<Self::Response, Box<dyn Error + Send + Sync>>;

    async fn handle_message<R, W>(
        &mut self,
        reader: &mut R,
        writer: &mut W,
    ) -> Result<(), Box<dyn Error + Send + Sync>>
    where
        R: AsyncReadExt + Unpin + Send,
        W: AsyncWriteExt + Unpin + Send,
    {
        // debug!("handling message");
        let req = Self::Request::decode_async(reader).await?;
        let resp = self.rpc(req).await?;
        resp.encode_async(writer).await?;
        Ok(())
    }
}

/// A trait for implementing a 9P service.
/// This trait is implemented for types that can handle 9P requests.
pub trait JetStreamSharedService:
    JetStreamAsyncService + Send + Sync + Clone
{
}

/// A service that implements the 9P protocol.
#[derive(Debug, Clone, Copy)]
pub struct JetStreamServiceAsyncImpl<S: JetStreamAsyncService> {
    inner: S,
}

impl<S: JetStreamSharedService> JetStreamServiceAsyncImpl<S> {
    pub fn new(inner: S) -> Self {
        JetStreamServiceAsyncImpl { inner }
    }
}

impl<S: JetStreamAsyncService> JetStreamProtocol
    for JetStreamServiceAsyncImpl<S>
{
    type Request = S::Request;
    type Response = S::Response;
}

impl<S: JetStreamSharedService + JetStreamAsyncService> JetStreamAsyncService
    for JetStreamServiceAsyncImpl<S>
{
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn rpc<'life0, 'async_trait>(
        &'life0 mut self,
        req: Self::Request,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<
                    Output = Result<
                        Self::Response,
                        Box<dyn Error + Send + Sync>,
                    >,
                > + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move { self.inner.rpc(req).await })
    }
}

pub struct JetStreamServiceImpl<S: JetStreamService> {
    inner: S,
}

impl<S: JetStreamService> JetStreamServiceImpl<S> {
    pub fn new(inner: S) -> Self {
        JetStreamServiceImpl { inner }
    }
}

impl<S: JetStreamService> JetStreamProtocol for JetStreamServiceImpl<S> {
    type Request = S::Request;
    type Response = S::Response;
}

#[derive(Debug)]
pub struct JetStreamRCService<T: Send + JetStreamAsyncService>(
    Arc<tokio::sync::Mutex<T>>,
);

impl<T: JetStreamAsyncService> JetStreamProtocol for JetStreamRCService<T> {
    type Request = T::Request;
    type Response = T::Response;
}

impl<T: Send + JetStreamAsyncService> JetStreamAsyncService
    for JetStreamRCService<T>
{
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn rpc<'life0, 'async_trait>(
        &'life0 mut self,
        req: Self::Request,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<
                    Output = Result<
                        Self::Response,
                        Box<dyn Error + Send + Sync>,
                    >,
                > + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move { self.0.lock().await.rpc(req).await })
    }
}

impl<T: Send + JetStreamAsyncService> JetStreamRCService<T> {
    pub fn new(inner: T) -> Self {
        JetStreamRCService(Arc::new(tokio::sync::Mutex::new(inner)))
    }
}

impl<T> Clone for JetStreamRCService<T>
where
    T: Send + JetStreamAsyncService,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> JetStreamSharedService for JetStreamRCService<T> where
    T: Send + JetStreamAsyncService
{
}

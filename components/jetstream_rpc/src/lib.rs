#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
//! # JetStream Rpc
//! Defines Rpc primitives for JetStream.
//! Of note is the `Protocol` trait which is meant to be used with the `service` attribute macro.
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
use jetstream_wireformat::{
    wire_format_extensions::AsyncWireFormatExt, WireFormat,
};
use std::fmt::Display;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// A trait representing a message that can be encoded and decoded.
pub trait Message: WireFormat + Send + Sync {}

/// Defines the request and response types for the JetStream protocol.
pub trait Protocol {
    type Request: Message;
    type Response: Message;
}

#[derive(Debug)]
pub enum Error {
    WireFormat,
    Io(std::io::Error),
    Anyhow(anyhow::Error),
    Quic(s2n_quic::connection::Error),
    Custom(Box<dyn std::error::Error + Send + Sync>),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Error::Anyhow(e)
    }
}

impl From<s2n_quic::connection::Error> for Error {
    fn from(e: s2n_quic::connection::Error) -> Self {
        Error::Quic(e)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {}", e),
            Error::WireFormat => write!(f, "Wire format error"),
            Error::Custom(e) => write!(f, "Custom error: {}", e),
            Error::Anyhow(e) => write!(f, "Anyhow error: {}", e),
            Error::Quic(e) => write!(f, "Quic error: {}", e),
        }
    }
}

/// An asynchronous JetStream service that can handle RPC calls and messages.
#[trait_variant::make(Send+Sync)]
pub trait Service: Protocol {
    /// Handles an RPC call asynchronously.
    async fn rpc(
        &mut self,
        req: Self::Request,
    ) -> Result<Self::Response, Error>;

    /// Handles a message by reading from the reader, processing it, and writing the response.
    async fn handle_message<R, W>(
        &mut self,
        reader: &mut R,
        writer: &mut W,
    ) -> Result<(), Error>
    where
        R: AsyncReadExt + Unpin + Send + Sync,
        W: AsyncWriteExt + Unpin + Send + Sync,
    {
        Box::pin(async move {
            let req = Self::Request::decode_async(reader).await?;
            let resp = self.rpc(req).await?;
            resp.encode_async(writer).await?;
            Ok(())
        })
    }
}

/// A shared, thread-safe JetStream service that can be cloned.
#[derive(Clone)]
pub struct SharedJetStreamService<S: Service> {
    inner: Arc<tokio::sync::Mutex<S>>,
}

impl<S: Service> SharedJetStreamService<S> {
    /// Creates a new shared JetStream service.
    pub fn new(service: S) -> Self {
        Self {
            inner: Arc::new(tokio::sync::Mutex::new(service)),
        }
    }
}

impl<S: Service> Protocol for SharedJetStreamService<S> {
    type Request = S::Request;
    type Response = S::Response;
}

impl<S: Service> Service for SharedJetStreamService<S> {
    async fn rpc(
        &mut self,
        req: Self::Request,
    ) -> Result<Self::Response, Error> {
        let mut service = self.inner.lock().await;
        service.rpc(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jetstream_wireformat::{
        wire_format_extensions::ConvertWireFormat, JetStreamWireFormat,
    };
    use okstd::prelude::*;
    use std::io::Cursor;

    struct TestService;

    #[derive(Debug, PartialEq, Eq, Clone, JetStreamWireFormat)]
    struct TestMessage {
        value: u32,
    }

    impl Message for TestMessage {}

    impl Protocol for TestService {
        type Request = TestMessage;
        type Response = TestMessage;
    }

    impl Service for TestService {
        #[doc = " Handles an RPC call asynchronously."]
        async fn rpc(
            &mut self,
            req: Self::Request,
        ) -> Result<Self::Response, Error> {
            Ok(TestMessage {
                value: req.value + 1,
            })
        }
    }

    #[okstd::test]
    async fn test_shared_service() {
        let mut service = SharedJetStreamService::new(TestService);
        let mut reader = Cursor::new(TestMessage { value: 42 }.to_bytes());
        let mut writer = Cursor::new(vec![]);

        service
            .handle_message(&mut reader, &mut writer)
            .await
            .unwrap();

        let resp =
            TestMessage::from_bytes(&bytes::Bytes::from(writer.into_inner()))
                .unwrap();
        assert_eq!(resp, TestMessage { value: 43 });
    }
}

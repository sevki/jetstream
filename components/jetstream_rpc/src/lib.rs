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

use jetstream_wireformat::WireFormat;

/// A trait representing a message that can be encoded and decoded.
pub trait Message: WireFormat + Send + Sync {}

/// Defines the request and response types for the JetStream protocol.
pub trait Protocol: Send + Sync {
    type Request: Message;
    type Response: Message;
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;

/// An asynchronous JetStream service that can handle RPC calls and messages.
#[trait_variant::make(Send + Sync + Sized)]
pub trait Service<P: Protocol> {
    /// Handles an RPC call asynchronously.
    async fn rpc(self, req: P::Request) -> Result<P::Response, Error>;
}

#[cfg(not(target_arch = "wasm32"))]
pub mod service {
    use std::sync::Arc;

    use crate::{Error, Protocol, Service};
    use jetstream_wireformat::wire_format_extensions::tokio::AsyncWireFormatExt;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// A shared, thread-safe JetStream service that can be cloned.
    #[derive(Clone)]
    pub struct SharedJetStreamService<P: Protocol + Clone, S: Service<P>> {
        inner: Arc<tokio::sync::Mutex<S>>,
        _protocol: std::marker::PhantomData<P>,
    }

    impl<P: Protocol + Clone, S: Service<P>> SharedJetStreamService<P, S> {
        /// Creates a new shared JetStream service.
        pub fn new(service: S) -> Self {
            Self {
                inner: Arc::new(tokio::sync::Mutex::new(service)),
                _protocol: std::marker::PhantomData,
            }
        }
    }

    impl<P: Protocol + Clone, S: Service<P>> Protocol
        for SharedJetStreamService<P, S>
    {
        type Request = P::Request;
        type Response = P::Response;
    }

    impl<P: Protocol + Clone, S: Service<P>> Service<P>
        for SharedJetStreamService<P, S>
    where
        S: Clone,
    {
        async fn rpc(self, req: P::Request) -> Result<P::Response, Error> {
            let mo = self.inner.lock_owned().await;
            mo.clone().rpc(req).await
        }
    }

    pub trait ServiceExt<P: Protocol>
    where
        Self: Service<P>,
    {
        /// Handles a message by reading from the reader, processing it, and writing the response.
        fn handle_message<R, W>(
            self,
            reader: &mut R,
            writer: &mut W,
        ) -> impl std::future::Future<Output = Result<(), Error>>
        where
            R: AsyncReadExt + Unpin + Send + Sync,
            W: AsyncWriteExt + Unpin + Send + Sync,
        {
            Box::pin(async move {
                let req = P::Request::decode_async(reader).await?;
                let resp = self.rpc(req).await?;
                resp.encode_async(writer).await?;
                Ok(())
            })
        }
    }
    impl<P: Protocol, S: Service<P>> ServiceExt<P> for S {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use jetstream_wireformat::{
        wire_format_extensions::ConvertWireFormat, JetStreamWireFormat,
    };
    use okstd::prelude::*;
    use service::SharedJetStreamService;
    use std::io::Cursor;
    #[derive(Debug, Clone)]
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

    impl Service<TestService> for TestService {
        #[doc = " Handles an RPC call asynchronously."]
        async fn rpc(
            self,
            req: <tests::TestService as Protocol>::Request,
        ) -> Result<<tests::TestService as Protocol>::Response, Error> {
            Ok(TestMessage {
                value: req.value + 1,
            })
        }
    }

    #[okstd::test]
    async fn test_shared_service() {
        use service::ServiceExt;
        let service = SharedJetStreamService::new(TestService);
        let mut reader = Cursor::new(TestMessage { value: 42 }.to_bytes());
        let mut writer = Cursor::new(vec![]);

        ServiceExt::handle_message(service, &mut reader, &mut writer)
            .await
            .unwrap();

        let resp =
            TestMessage::from_bytes(&bytes::Bytes::from(writer.into_inner()))
                .unwrap();
        assert_eq!(resp, TestMessage { value: 43 });
    }
}

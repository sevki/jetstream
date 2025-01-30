//! Service
use {jetstream::prelude::*, jetstream_rpc::Protocol, jetstream_wireformat::JetStreamWireFormat};

trait _RemoteService<P: Protocol>: Send + Sync + Unpin + Sized {}

/// Service
#[derive(Debug)]
pub struct Service<P: Protocol> {
    #[allow(dead_code)]
    inner: P,
}

#[derive(Debug, JetStreamWireFormat)]
struct AnyService {}

#[service]
trait RemoteService {
    async fn ping(&mut self) -> Result<crate::service::AnyService, Error>;
}

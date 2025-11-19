use async_trait::async_trait;
use bytes::Bytes;
use jetstream_wireformat::wire_format_extensions::ConvertWireFormat;

use crate::{context, Frame, Protocol};

use super::Error;

#[async_trait]
pub trait DynamicProtocol: Send + Sync {
    fn protocol_version(&self) -> &'static str;
    async fn rpc(
        &mut self,
        context: context::Context,
        data: &[u8],
    ) -> Result<Vec<u8>, Error>;
}

#[async_trait]
impl<P: Protocol> DynamicProtocol for P {
    fn protocol_version(&self) -> &'static str {
        P::VERSION
    }

    async fn rpc(
        &mut self,
        context: context::Context,
        data: &[u8],
    ) -> Result<Vec<u8>, Error> {
        let frame =
            Frame::<P::Request>::from_bytes(&Bytes::copy_from_slice(data))?;
        Ok(self
            .rpc(context, frame)
            .await
            .map_err(|e| Error::Generic(e.into()))?
            .as_bytes())
    }
}

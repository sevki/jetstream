use std::fmt::Debug;

use futures::{SinkExt, StreamExt};
use iroh::{endpoint::Connection, protocol::ProtocolHandler};
use jetstream_rpc::IntoError;
use jetstream_rpc::{
    context::{Context, NodeId},
    server::ServerCodec,
    Protocol,
};
use tokio_util::codec::{FramedRead, FramedWrite};

#[derive(Debug)]
pub struct IrohServer<P: Protocol + Debug + Clone + 'static> {
    inner: P,
}

impl<P: Protocol + Debug + Clone + 'static> IrohServer<P> {
    pub fn new(protocol: P) -> Self {
        IrohServer { inner: protocol }
    }
}

impl<P: Protocol + Debug + Clone + 'static> ProtocolHandler for IrohServer<P> {
    async fn accept(
        &self,
        connection: Connection,
    ) -> Result<(), iroh::protocol::AcceptError> {
        let (send_stream, recv_stream) = connection.accept_bi().await?;
        let mut handler = self.inner.clone();
        tokio::spawn(async move {
            let mut reader =
                FramedRead::new(recv_stream, ServerCodec::<P>::new());
            let mut writer =
                FramedWrite::new(send_stream, ServerCodec::<P>::new());
            while let Some(req) = reader.next().await {
                let node_id: NodeId = connection
                    .remote_node_id()
                    .expect("Failed to get remote node ID")
                    .into();
                let context = Context::from(node_id);
                match req {
                    Ok(req) => match handler.rpc(context, req).await {
                        Ok(resp) => {
                            writer.send(resp).await.unwrap();
                        }
                        Err(err) => {
                            let error = err.into_error();
                            eprintln!("Error processing request: {}", error);
                        }
                    },
                    Err(_err) => {}
                }
            }
        });
        Ok(())
    }
}

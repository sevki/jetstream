use std::fmt::Debug;

use futures::{SinkExt, StreamExt};
use iroh::{endpoint::Connection, protocol::ProtocolHandler};
use jetstream_rpc::server::Server;
use jetstream_rpc::IntoError;
use jetstream_rpc::{
    context::{Context, NodeId},
    server::ServerCodec,
    Frame, Protocol,
};
use tokio::sync::mpsc;
use tokio_util::codec::{FramedRead, FramedWrite};

#[derive(Debug)]
pub struct IrohServer<P: Protocol + Server + Debug + Clone + 'static> {
    inner: P,
}

impl<P: Protocol + Server + Debug + Clone + Send + Sync + 'static>
    IrohServer<P>
{
    pub fn new(protocol: P) -> Self {
        IrohServer { inner: protocol }
    }
}

impl<P: Protocol + Server + Debug + Clone + 'static> ProtocolHandler
    for IrohServer<P>
{
    async fn accept(
        &self,
        connection: Connection,
    ) -> Result<(), iroh::protocol::AcceptError> {
        let (send_stream, recv_stream) = connection.accept_bi().await?;
        let handler = self.inner.clone();
        let node_id: NodeId = connection
            .remote_node_id()
            .expect("Failed to get remote node ID")
            .into();

        tokio::spawn(async move {
            let mut reader =
                FramedRead::new(recv_stream, ServerCodec::<P>::new());
            let mut writer =
                FramedWrite::new(send_stream, ServerCodec::<P>::new());

            // Channel for sending responses back to the writer
            let (resp_tx, mut resp_rx) =
                mpsc::channel::<Frame<P::Response>>(256);

            // Spawn a task to write responses as they complete
            let writer_task = tokio::spawn(async move {
                while let Some(resp) = resp_rx.recv().await {
                    if writer.send(resp).await.is_err() {
                        break;
                    }
                }
            });

            // Process requests concurrently
            while let Some(req) = reader.next().await {
                let context = Context::from(node_id.clone());
                match req {
                    Ok(req) => {
                        let mut handler = handler.clone();
                        let resp_tx = resp_tx.clone();
                        tokio::spawn(async move {
                            match handler.rpc(context, req).await {
                                Ok(resp) => {
                                    let _ = resp_tx.send(resp).await;
                                }
                                Err(err) => {
                                    let error = err.into_error();
                                    eprintln!(
                                        "Error processing request: {}",
                                        error
                                    );
                                }
                            }
                        });
                    }
                    Err(_err) => {}
                }
            }

            // Drop the sender so the writer task knows to finish
            drop(resp_tx);
            let _ = writer_task.await;
        });
        Ok(())
    }
}

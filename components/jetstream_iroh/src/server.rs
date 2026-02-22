use std::fmt::Debug;
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use iroh::{endpoint::Connection, protocol::ProtocolHandler};
use jetstream_rpc::{
    context::{Context, NodeId},
    server::{Server, ServerCodec},
    Frame, IntoError, Protocol, Router as RpcRouter,
};
use tokio::io::{AsyncRead, AsyncWrite};
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
        let handler = self.inner.clone();
        let node_id: NodeId = connection
            .remote_node_id()
            .expect("Failed to get remote node ID")
            .into();

        loop {
            let (send_stream, recv_stream) = match connection.accept_bi().await
            {
                Ok(streams) => streams,
                Err(_) => break,
            };
            let handler = handler.clone();
            let node_id = node_id.clone();
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
        }
        Ok(())
    }
}

/// An Iroh protocol handler that uses an `RpcRouter` for per-stream
/// version-based protocol dispatch.
#[derive(Debug)]
pub struct IrohRouter {
    pub router: Arc<RpcRouter>,
}

impl IrohRouter {
    pub fn new(router: Arc<RpcRouter>) -> Self {
        Self { router }
    }
}

impl ProtocolHandler for IrohRouter {
    async fn accept(
        &self,
        connection: Connection,
    ) -> Result<(), iroh::protocol::AcceptError> {
        let router = self.router.clone();
        let node_id: NodeId = connection
            .remote_node_id()
            .expect("Failed to get remote node ID")
            .into();

        loop {
            let (send_stream, recv_stream) = match connection.accept_bi().await
            {
                Ok(streams) => streams,
                Err(_) => break,
            };
            let router = router.clone();
            let ctx = Context::from(node_id.clone());
            tokio::spawn(async move {
                let reader: Box<dyn AsyncRead + Send + Sync + Unpin> =
                    Box::new(recv_stream);
                let writer: Box<dyn AsyncWrite + Send + Sync + Unpin> =
                    Box::new(send_stream);
                if let Err(e) = router.accept(ctx, reader, writer).await {
                    eprintln!("Iroh router dispatch error: {}", e);
                }
            });
        }
        Ok(())
    }
}

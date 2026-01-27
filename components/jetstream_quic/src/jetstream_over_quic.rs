use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use jetstream_rpc::{
    context::Context,
    server::{Server, ServerCodec},
    Frame, IntoError,
};
use quinn::Connection;
use tokio::sync::mpsc;
use tokio_util::codec::{FramedRead, FramedWrite};

use crate::ProtocolHandler;

#[async_trait]
impl<T> ProtocolHandler for T
where
    T: Server<Error = jetstream_rpc::Error> + Clone + 'static,
{
    fn alpn(&self) -> String {
        Self::VERSION.to_string()
    }
    async fn accept(&self, ctx: Context, conn: Connection) -> () {
        let handler = self.clone();
        while let Ok((send, recv)) = conn.accept_bi().await {
            let handler = handler.clone();
            let ctx = ctx.clone();
            tokio::spawn(async move {
                let mut reader = FramedRead::new(recv, ServerCodec::<T>::new());
                let mut writer =
                    FramedWrite::new(send, ServerCodec::<T>::new());

                // Channel for sending responses back to the writer
                let (resp_tx, mut resp_rx) =
                    mpsc::channel::<Frame<T::Response>>(256);

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
                    let ctx = ctx.clone();
                    match req {
                        Ok(req) => {
                            let mut handler = handler.clone();
                            let resp_tx = resp_tx.clone();
                            tokio::spawn(async move {
                                match handler.rpc(ctx, req).await {
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
                    };
                }

                // Drop the sender so the writer task knows to finish
                drop(resp_tx);
                let _ = writer_task.await;
            });
        }
    }
}

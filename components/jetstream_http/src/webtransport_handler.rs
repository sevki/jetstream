use async_trait::async_trait;
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use h3::quic;
use h3_webtransport::server::{AcceptedBi, WebTransportSession};
use jetstream_rpc::{
    server::{Server, ServerCodec},
    Error, Frame, IntoError,
};
use tokio_util::codec::{FramedRead, FramedWrite};

// r[impl jetstream.webtransport.handler-trait]
#[async_trait]
pub trait WebTransportHandler: Send + Sync {
    async fn handle_session(
        &self,
        session: WebTransportSession<h3_quinn::Connection, Bytes>,
        ctx: jetstream_rpc::context::Context,
    ) -> jetstream_error::Result<()>;
}

// r[impl jetstream.webtransport.upstream-initiated]
/// Opens a server-initiated bidirectional stream to the downstream client.
///
/// This allows the server to push RPC frames to the client over a new bidi stream.
/// The returned writer/reader pair uses `ServerCodec` for frame encoding/decoding.
///
/// # r[impl jetstream.webtransport.upstream-initiated.use-case]
/// Use this for server-push scenarios where the server needs to initiate
/// communication with the client (e.g., pushing notifications, streaming updates).
pub async fn open_upstream_bidi<T>(
    session: &WebTransportSession<h3_quinn::Connection, Bytes>,
    frame: Frame<T::Response>,
) -> Result<Frame<T::Request>, Error>
where
    T: Server<Error = Error> + Send + Sync + Clone + 'static,
{
    let session_id = session.session_id();
    let stream = session
        .open_bi(session_id)
        .await
        .map_err(|e| Error::from(std::io::Error::other(e.to_string())))?;
    let (send, recv) = quic::BidiStream::split(stream);
    let mut writer = FramedWrite::new(send, ServerCodec::<T>::new());
    let mut reader = FramedRead::new(recv, ServerCodec::<T>::new());

    writer.send(frame).await?;

    match reader.next().await {
        Some(Ok(response)) => Ok(response),
        Some(Err(e)) => Err(e),
        None => Err(Error::from(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "upstream bidi stream closed before response",
        ))),
    }
}

// r[impl jetstream.webtransport.bidi]
// r[impl jetstream.webtransport.bidi.concurrent]
// r[impl jetstream.webtransport.bidi.multi-stream]
// r[impl jetstream.webtransport.lifecycle]
// r[impl jetstream.webtransport.errors]
// r[impl jetstream.webtransport.errors.session]
#[async_trait]
impl<T> WebTransportHandler for T
where
    T: Server<Error = jetstream_rpc::Error> + Send + Sync + Clone + 'static,
{
    async fn handle_session(
        &self,
        session: WebTransportSession<h3_quinn::Connection, Bytes>,
        ctx: jetstream_rpc::context::Context,
    ) -> jetstream_error::Result<()> {
        let handler = self.clone();
        tokio::spawn(async move {
            loop {
                match session.accept_bi().await {
                    Ok(Some(AcceptedBi::BidiStream(_, stream))) => {
                        let (send, recv) = quic::BidiStream::split(stream);
                        let handler = handler.clone();
                        let ctx = ctx.clone();
                        tokio::spawn(async move {
                            let mut reader =
                                FramedRead::new(recv, ServerCodec::<T>::new());
                            let mut writer =
                                FramedWrite::new(send, ServerCodec::<T>::new());

                            // Channel for sending responses back to the writer
                            let (resp_tx, mut resp_rx) =
                                tokio::sync::mpsc::channel::<Frame<T::Response>>(
                                    256,
                                );

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
                                                    let _ = resp_tx
                                                        .send(resp)
                                                        .await;
                                                }
                                                Err(err) => {
                                                    let error =
                                                        err.into_error();
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
                    Ok(Some(_)) => continue,
                    Ok(None) => break,
                    Err(err) => {
                        eprintln!("Error accepting bidi stream: {}", err);
                        break;
                    }
                }
            }
        });
        Ok(())
    }
}

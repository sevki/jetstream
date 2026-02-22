use crate::{
    context::Context,
    server::{Server, ServerCodec},
    version::VersionFrame,
    Error, Frame, Protocol, Rversion, Version,
};
use async_trait::async_trait;
use futures::SinkExt;
use futures::StreamExt;
use jetstream_error::IntoError;
use std::{collections::HashMap, str::FromStr, sync::Arc};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::mpsc,
};
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::{error, info};

pub trait Incoming: AsyncRead + AsyncWrite + Send + Sync {}

#[async_trait]
pub trait Handler: Send + Sync {
    async fn handle(
        &self,
        ctx: Context,
        reader: Box<dyn AsyncRead + Send + Sync + Unpin>,
        writer: Box<dyn AsyncWrite + Send + Sync + Unpin>,
    ) -> Result<(), Error>;
}

#[derive(Clone)]
pub struct Router {
    handlers: HashMap<String, Arc<Box<dyn Handler>>>,
}

impl std::fmt::Debug for Router {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Router")
            .field("handlers", &self.handlers.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Router {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a handler for a protocol name.
    /// The name should match the protocol name portion from
    /// `rs.jetstream.proto/{name}/{version}`, or `9P2000`/`9P2000.L` for legacy protocols.
    pub fn with_handler(
        mut self,
        name: &str,
        handler: impl Handler + 'static,
    ) -> Self {
        self.handlers
            .insert(name.to_string(), Arc::new(Box::new(handler)));
        self
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

struct VersionProtocol {}

impl Protocol for VersionProtocol {
    type Request = crate::version::VersionFrame;

    type Response = crate::version::VersionFrame;

    type Error = Error;

    const VERSION: &'static str = "";

    const NAME: &'static str = "";
}

impl Router {
    pub async fn accept(
        &self,
        ctx: Context,
        reader: Box<dyn AsyncRead + Send + Sync + Unpin>,
        writer: Box<dyn AsyncWrite + Send + Sync + Unpin>,
    ) -> Result<(), Error> {
        let mut framed_read =
            FramedRead::new(reader, ServerCodec::<VersionProtocol>::new());
        let mut framed_write =
            FramedWrite::new(writer, ServerCodec::<VersionProtocol>::new());

        let version = framed_read.next().await;
        let Some(frame) = version else {
            return Err(Error::with_code(
                "connection closed before version negotiation",
                "jetstream_rpc::error::no_version",
            ));
        };
        info!("received version frame: {:?}", frame);
        let frame = frame?;
        {
            match frame.msg {
                VersionFrame::Tversion(tversion) => {
                    let version: Version = Version::from_str(&tversion.version)
                        .map_err(|e| {
                            Error::with_code(
                                e,
                                "jetstream_rpc::error::version_negotiation",
                            )
                        })?;
                    let (handler, version) = match version {
                        Version::V9P2000L => {
                            if let Some(handler) = self
                                .handlers
                                .get(&Version::V9P2000L.to_string())
                            {
                                (handler, version)
                            } else {
                                return Err(
                                    Error::with_code(
                                        "handler not found",
                                        "jetstream_rpc::error::jetstream_handler_not_found",
                                    )
                                );
                            }
                        }
                        Version::V9P2000 => {
                            if let Some(handler) =
                                self.handlers.get(&Version::V9P2000.to_string())
                            {
                                (handler, version)
                            } else {
                                return Err(
                                    Error::with_code(
                                        "handler not found",
                                        "jetstream_rpc::error::jetstream_handler_not_found",
                                    )
                                );
                            }
                        }
                        Version::JetStream { ref name, .. } => {
                            if let Some(handler) = self.handlers.get(name) {
                                (handler, version)
                            } else {
                                return Err(
                                    Error::with_code(
                                        "handler not found",
                                        "jetstream_rpc::error::jetstream_handler_not_found",
                                    )
                                );
                            }
                        }
                    };
                    framed_write
                        .send(Frame {
                            tag: frame.tag,
                            msg: VersionFrame::Rversion(Rversion {
                                msize: tversion.msize,
                                version: version.to_string(),
                            }),
                        })
                        .await?;

                    let reader = framed_read.into_inner();
                    let writer = framed_write.into_inner();
                    handler.handle(ctx, reader, writer).await?;
                }
                VersionFrame::Rversion(_) => {
                    return Err(Error::with_code(
                        "Client sent Rversion instead of Tversion",
                        "jetstream_rpc::error::unexpected_rversion",
                    ))
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl<T: Server + Clone + 'static> Handler for T {
    async fn handle(
        &self,
        ctx: Context,
        reader: Box<dyn AsyncRead + Send + Sync + Unpin>,
        writer: Box<dyn AsyncWrite + Send + Sync + Unpin>,
    ) -> Result<(), Error> {
        let server = self.clone();
        tokio::spawn(async move {
            let mut reader = FramedRead::new(reader, ServerCodec::<T>::new());
            let mut writer = FramedWrite::new(writer, ServerCodec::<T>::new());

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
                        let mut handler = server.clone();
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
                    Err(err) => {
                        error!("Error decoding request frame: {}", err);
                    }
                };
            }

            // Drop the sender so the writer task knows to finish
            drop(resp_tx);
            let _ = writer_task.await;
        });
        Ok(())
    }
}

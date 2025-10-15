use std::{fmt::Debug, io};

use futures::{SinkExt, StreamExt};
use iroh::{endpoint::Connection, protocol::ProtocolHandler};
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
                            eprintln!("Error processing request: {}", err);
                        }
                    },
                    Err(err) => match err {
                        jetstream_rpc::Error::Io(error) => match error.kind() {
                            io::ErrorKind::NotFound => todo!(),
                            io::ErrorKind::PermissionDenied => todo!(),
                            io::ErrorKind::ConnectionRefused => todo!(),
                            io::ErrorKind::ConnectionReset => todo!(),
                            io::ErrorKind::HostUnreachable => todo!(),
                            io::ErrorKind::NetworkUnreachable => todo!(),
                            io::ErrorKind::ConnectionAborted => todo!(),
                            io::ErrorKind::NotConnected => {
                                return;
                            }
                            io::ErrorKind::AddrInUse => todo!(),
                            io::ErrorKind::AddrNotAvailable => todo!(),
                            io::ErrorKind::NetworkDown => todo!(),
                            io::ErrorKind::BrokenPipe => todo!(),
                            io::ErrorKind::AlreadyExists => todo!(),
                            io::ErrorKind::WouldBlock => todo!(),
                            io::ErrorKind::NotADirectory => todo!(),
                            io::ErrorKind::IsADirectory => todo!(),
                            io::ErrorKind::DirectoryNotEmpty => todo!(),
                            io::ErrorKind::ReadOnlyFilesystem => todo!(),
                            io::ErrorKind::StaleNetworkFileHandle => todo!(),
                            io::ErrorKind::InvalidInput => todo!(),
                            io::ErrorKind::InvalidData => todo!(),
                            io::ErrorKind::TimedOut => todo!(),
                            io::ErrorKind::WriteZero => todo!(),
                            io::ErrorKind::StorageFull => todo!(),
                            io::ErrorKind::NotSeekable => todo!(),
                            io::ErrorKind::QuotaExceeded => todo!(),
                            io::ErrorKind::FileTooLarge => todo!(),
                            io::ErrorKind::ResourceBusy => todo!(),
                            io::ErrorKind::ExecutableFileBusy => todo!(),
                            io::ErrorKind::Deadlock => todo!(),
                            io::ErrorKind::CrossesDevices => todo!(),
                            io::ErrorKind::TooManyLinks => todo!(),
                            io::ErrorKind::InvalidFilename => todo!(),
                            io::ErrorKind::ArgumentListTooLong => todo!(),
                            io::ErrorKind::Interrupted => todo!(),
                            io::ErrorKind::Unsupported => todo!(),
                            io::ErrorKind::UnexpectedEof => todo!(),
                            io::ErrorKind::OutOfMemory => todo!(),
                            io::ErrorKind::Other => todo!(),
                            _ => todo!(),
                        },
                        jetstream_rpc::Error::Generic(_error) => todo!(),
                        jetstream_rpc::Error::Custom(_) => todo!(),
                        jetstream_rpc::Error::InvalidResponse => todo!(),
                    },
                }
            }
        });
        Ok(())
    }
}

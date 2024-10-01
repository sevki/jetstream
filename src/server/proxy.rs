use std::{fmt::Debug, net::SocketAddr, path::Path};

use crate::{
    client::{self, DialQuic},
    coding::{Rframe, Tframe},
};
use anyhow::Ok;
use s2n_quic::{
    client::{Client, Connect},
    provider::tls,
};

use okstd::prelude::*;

use crate::wire_format_extensions::AsyncWireFormatExt;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio_vsock::{VsockAddr, VsockListener};

#[async_trait::async_trait]
pub trait ListenerStream: Send + Sync + Debug + 'static {
    type Stream: AsyncRead + AsyncWrite + Unpin + Send + Sync;
    type Addr: std::fmt::Debug;
    async fn accept(&mut self) -> std::io::Result<(Self::Stream, Self::Addr)>;
}

#[async_trait::async_trait]
impl ListenerStream for tokio::net::UnixListener {
    type Stream = tokio::net::UnixStream;
    type Addr = tokio::net::unix::SocketAddr;
    async fn accept(&mut self) -> std::io::Result<(Self::Stream, Self::Addr)> {
        tokio::net::UnixListener::accept(self).await
    }
}

#[async_trait::async_trait]
impl ListenerStream for VsockListener {
    type Stream = tokio_vsock::VsockStream;
    type Addr = VsockAddr;
    async fn accept(&mut self) -> std::io::Result<(Self::Stream, Self::Addr)> {
        VsockListener::accept(self).await
    }
}

pub struct Proxy<L>
where
    L: ListenerStream,
{
    dial: DialQuic,
    listener: L,
}

impl<L> Proxy<L>
where
    L: ListenerStream,
{
    pub fn new(dial: DialQuic, listener: L) -> Self {
        Self { dial, listener }
    }

    pub async fn run(&mut self) {
        debug!("Listening on {:?}", self.listener);
        while let std::result::Result::Ok((down_stream, _)) =
            self.listener.accept().await
        {
            debug!("Accepted connection from");
            let down_stream = down_stream;
            let dial = self.dial.clone();
            tokio::spawn(async move {
                debug!("Dialing {:?}", dial);
                let mut dial = dial.clone().dial().await.unwrap();
                debug!("Connected to {:?}", dial.remote_addr());
                let up_stream = dial.open_bidirectional_stream().await.unwrap();
                let (rx, mut tx) = up_stream.split();
                let (read, mut write) = tokio::io::split(down_stream);
                let mut upstream_reader = tokio::io::BufReader::new(rx);
                let mut downstream_reader = tokio::io::BufReader::new(read);
                loop {
                    // read and send to up_stream
                    {
                        debug!("Reading from down_stream");
                        let tframe =
                            Tframe::decode_async(&mut downstream_reader).await;
                        if let Err(e) = tframe {
                            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                                break;
                            } else {
                                error!(
                                    "Error decoding from down_stream: {:?}",
                                    e
                                );
                                break;
                            }
                        } else if let std::io::Result::Ok(tframe) = tframe {
                            debug!("Sending to up_stream {:?}", tframe);
                            tframe.encode_async(&mut tx).await.unwrap();
                        }
                    }
                    // write and send to down_stream
                    {
                        debug!("Reading from up_stream");
                        let rframe = Rframe::decode_async(&mut upstream_reader)
                            .await
                            .unwrap();
                        debug!("Sending to down_stream");
                        rframe.encode_async(&mut write).await.unwrap();
                    }
                }
            });
        }
    }
}

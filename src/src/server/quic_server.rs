use std::{path::Path, sync::Arc};

use bytes::Bytes;

use futures_util::AsyncReadExt;
use p9::{Tframe, WireFormat};
use s2n_quic::{provider::tls, Server};
use serde::de;
use slog_scope::{debug, error};
use tokio::sync::Mutex;
use tower::Service;

use crate::{
    async_wire_format::AsyncWireFormatExt,
    log::{self, setup_logging},
    JetStreamService, Message, NinePService, NinePServiceImpl, Radar,
};

pub struct QuicServer<
    Req: Message,
    Resp: Message,
    S: JetStreamService<Req, Resp>,
> {
    svc: S,
    _ghost: std::marker::PhantomData<(Req, Resp)>,
}

impl<Req: Message, Resp: Message, S: JetStreamService<Req, Resp>>
    QuicServer<Req, Resp, S>
{
    pub fn new(svc: S) -> Self {
        Self {
            svc,
            _ghost: std::marker::PhantomData,
        }
    }
}

impl<
        Req: Message,
        Resp: Message,
        T: JetStreamService<Req, Resp> + Clone + 'static,
    > QuicServer<Req, Resp, T>
{
    pub async fn serve(mut self, mut server: Server) -> anyhow::Result<()> {
        debug!("Server started");
        while let Some(mut connection) = server.accept().await {
            debug!("Connection opened from {:?}", connection.remote_addr());
            let svc = self.svc.clone();
            // spawn a new task for the connection
            tokio::spawn(async move {
                debug!("Connection opened from {:?}", connection.remote_addr());
                let svc = svc.clone();
                while let Ok(Some(stream)) =
                    connection.accept_bidirectional_stream().await
                {
                    // spawn a new task for the stream
                    let svc = svc.clone();
                    tokio::spawn(async move {
                        debug!("Stream opened");
                        // echo any data back to the stream

                        let (read, mut write) = stream.split();
                        // let mut downstream_writer =
                        //     tokio::io::BufWriter::new(write);
                        let mut downstream_reader =
                            tokio::io::BufReader::new(read);

                        let mut svc = svc.clone();
                        loop {
                            // read and send to up_stream
                            {
                                debug!("Reading from down_stream");
                                let tframe =
                                    Req::decode_async(&mut downstream_reader)
                                        .await;
                                //debug!("got tframe: {:?}", tframe);
                                if let Err(e) = tframe {
                                    // if error is eof, break
                                    if e.kind()
                                        == std::io::ErrorKind::UnexpectedEof
                                    {
                                        break;
                                    } else {
                                        error!(
                                            "Error decoding from down_stream: {:?}",
                                            e
                                        );
                                        break;
                                    }
                                } else if let std::io::Result::Ok(tframe) =
                                    tframe
                                {
                                    debug!("Sending to up_stream");
                                    let rframe =
                                        svc.clone().call(tframe).await.unwrap();
                                    //debug!("got rframe: {:?}", rframe);
                                    debug!("writing to down_stream");
                                    rframe
                                        .encode_async(&mut write)
                                        .await
                                        .unwrap();
                                    write.flush().await.unwrap();
                                }
                            }
                        }
                    });
                }
            });
        }
        Ok(())
    }
}

use s2n_quic::Server;

use crate::{
    service::{JetStreamProtocol, JetStreamSharedService},
    wire_format_extensions::AsyncWireFormatExt,
};

use okstd::prelude::*;

use crate::service::{JetStreamAsyncService, Message};

pub struct QuicServer<S: JetStreamAsyncService + JetStreamSharedService> {
    svc: S,
}

impl<S: JetStreamAsyncService + JetStreamSharedService> QuicServer<S> {
    pub fn new(svc: S) -> Self {
        Self { svc }
    }
}

impl<S: JetStreamAsyncService + JetStreamSharedService + 'static>
    QuicServer<S>
{
    pub async fn serve(self, mut server: Server) -> anyhow::Result<()> {
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

                        let svc = svc.clone();
                        loop {
                            // read and send to up_stream
                            {
                                // debug!("handling message");
                                let _res = svc
                                    .clone()
                                    .handle_message(
                                        &mut downstream_reader,
                                        &mut write,
                                    )
                                    .await;
                                if _res.is_err() {
                                    error!(
                                        "Error handling message: {:?}",
                                        _res
                                    );
                                    break;
                                }
                                debug!("Reading from down_stream");
                                let tframe =
                                    <S as JetStreamProtocol>::Request::decode_async(&mut downstream_reader)
                                        .await;
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
                                    let rframe = JetStreamAsyncService::rpc(
                                        &mut svc.clone(),
                                        tframe,
                                    )
                                    .await
                                    .unwrap();
                                    // debug!("got rframe: {:?}", rframe);
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

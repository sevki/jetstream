use std::path::Path;

// Copyright (c) 2024, Sevki <s@sevki.io>
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.
use s2n_quic::{provider::tls, Server};

use jetstream_wireformat::wire_format_extensions::AsyncWireFormatExt;

use okstd::prelude::*;

use jetstream_rpc::{Protocol, Service, SharedJetStreamService};

pub struct QuicServer<S: Service> {
    svc: S,
}

impl<S: Service> QuicServer<S> {
    pub fn new(svc: S) -> Self {
        Self { svc }
    }
}

impl<S> QuicServer<S>
where
    S: Service + Clone + 'static,
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
                                    <S as Protocol>::Request::decode_async(
                                        &mut downstream_reader,
                                    )
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
                                    let rframe =
                                        Service::rpc(&mut svc.clone(), tframe)
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

pub async fn start_server(svc: impl Service + Clone + 'static) {
    pub static CA_CERT_PEM: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/certs/ca-cert.pem");
    pub static SERVER_CERT_PEM: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/certs/server-cert.pem");
    pub static SERVER_KEY_PEM: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/certs/server-key.pem");

    let tls = tls::default::Server::builder()
        .with_trusted_certificate(Path::new(CA_CERT_PEM))
        .unwrap()
        .with_certificate(Path::new(SERVER_CERT_PEM), Path::new(SERVER_KEY_PEM))
        .unwrap()
        .with_client_authentication()
        .unwrap()
        .build()
        .unwrap();

    let server = Server::builder()
        .with_tls(tls)
        .unwrap()
        .with_io("127.0.0.1:4433")
        .unwrap()
        .start()
        .unwrap();

    let qsrv = QuicServer::new(SharedJetStreamService::new(svc));

    qsrv.serve(server).await.unwrap();
}

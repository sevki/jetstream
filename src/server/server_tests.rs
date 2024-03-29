#[cfg(test)]
mod tests {
    use crate::{
        async_wire_format::AsyncWireFormatExt,
        log::{drain, setup_logging},
        server::{
            proxy::{DialQuic, Proxy},
            quic_server::QuicServer,
        }, service::JetStreamService, messages::Rmessage,
    };
    use crate::protocol::{Rframe, Tframe, Tmessage, Tversion};
    use futures_util::Future;
    use s2n_quic::{provider::tls, Server};
    use slog_scope::debug;
    use std::{
        path::{self, Path},
        sync::Arc, pin::Pin, error::Error,
    };
    use tokio::{io::AsyncWriteExt, sync::Barrier, net::UnixListener};

    #[derive(Debug, Clone)]
    pub struct EchoService;

    impl JetStreamService<Tframe, Rframe> for EchoService {
        fn call(
            &mut self,
            _req: Tframe,
        ) -> Pin<
            Box<
                dyn Future<
                        Output = Result<Rframe, Box<dyn Error + Send + Sync>>,
                    > + Send,
            >,
        > {
            Box::pin(async move {
                Ok(Rframe {
                    tag: 0,
                    msg: Rmessage::Version(crate::protocol::Rversion {
                        msize: 0,
                        version: "9P2000".to_string(),
                    }),
                })
            })
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_quic_server_unix_socket_proxy() {
        let _guard = slog_scope::set_global_logger(setup_logging());
        let _guard = slog_envlogger::new(drain());
        let (_tx, _rx) = tokio::io::duplex(1024);
        pub static CA_CERT_PEM: &str =
            concat!(env!("CARGO_MANIFEST_DIR"), "/certs/ca-cert.pem");
        pub static SERVER_CERT_PEM: &str =
            concat!(env!("CARGO_MANIFEST_DIR"), "/certs/server-cert.pem");
        pub static SERVER_KEY_PEM: &str =
            concat!(env!("CARGO_MANIFEST_DIR"), "/certs/server-key.pem");
        pub static CLIENT_CERT_PEM: &str =
            concat!(env!("CARGO_MANIFEST_DIR"), "/certs/client-cert.pem");
        pub static CLIENT_KEY_PEM: &str =
            concat!(env!("CARGO_MANIFEST_DIR"), "/certs/client-key.pem");

        let tls = tls::default::Server::builder()
            .with_trusted_certificate(Path::new(CA_CERT_PEM))
            .unwrap()
            .with_certificate(
                Path::new(SERVER_CERT_PEM),
                Path::new(SERVER_KEY_PEM),
            )
            .unwrap()
            .with_client_authentication()
            .unwrap()
            .build()
            .unwrap();

        let barrier = Arc::new(Barrier::new(3)).clone();
        let c = barrier.clone();
        let srv_handle = tokio::spawn(async move {
            let server = Server::builder()
                .with_tls(tls)
                .unwrap()
                .with_io("127.0.0.1:4433")
                .unwrap()
                .start()
                .unwrap();
            let qsrv: QuicServer<Tframe, Rframe, EchoService> =
                QuicServer::new(EchoService);
            debug!("Server started, waiting for barrier");
            c.wait().await;
            let _ = qsrv.serve(server).await;
        });

        let cert = path::PathBuf::from(CLIENT_CERT_PEM).into_boxed_path();
        let key = path::PathBuf::from(CLIENT_KEY_PEM).into_boxed_path();
        let ca_cert = path::PathBuf::from(CA_CERT_PEM).into_boxed_path();
        let temp_dir = tmpdir::TmpDir::new("q9p").await.unwrap();

        let mut listen = temp_dir.to_path_buf();
        listen.push("q9p.sock");
        let listen = listen.into_boxed_path();
        let l = UnixListener::bind(listen.clone()).unwrap();
        let c = barrier.clone();

        let prxy_handle = tokio::spawn(async move {
            debug!("Proxy started, waiting for barrier");
            c.wait().await;

            let mut prxy = Proxy::new(
                DialQuic::new(
                    "127.0.0.1".to_string(),
                    4433,
                    cert,
                    key,
                    ca_cert,
                    "localhost".to_string(),
                ),
                l,
            );
            let _ = prxy.run().await;
        });
        let c = barrier.clone();
        let l = listen.clone();
        let client_handle = tokio::spawn(async move {
            c.clone().wait().await;
            // sleep for 5 milliseconds to give the server time to start
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            debug!("Connecting to {:?}", listen);
            let (mut read, mut write) = tokio::net::UnixStream::connect(l)
                .await
                .unwrap()
                .into_split();

            // loop 100 times
            for _ in 0..100 {
                let test = Tframe {
                    tag: 0,
                    msg: Ok(Tmessage::Version(Tversion {
                        msize: 8192,
                        version: "9P2000.L".to_string(),
                    })),
                };
                debug!("Sending tframe: {:?}", test);
                // ping
                test.encode_async(&mut write).await.unwrap();
                write.flush().await.unwrap();
                debug!("Reading rframe");
                read.readable().await.unwrap();
                // pong
                let resp = Rframe::decode_async(&mut read).await.unwrap();
                assert_eq!(resp.tag, 0);
            }
        });

        let timeout = std::time::Duration::from_secs(10);

        let timeout = tokio::time::sleep(timeout);

        tokio::select! {
            _ = timeout => {
                panic!("Timeout");
            }
            _ = srv_handle => {
                panic!("Quic server failed");
            }
            _ = prxy_handle => {
                panic!("Proxy failed");
            }
            _ = client_handle => {
                return;
            }
        }
    }
}

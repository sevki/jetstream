use std::sync::Arc;

use bytes::Bytes;
use h3_quinn::quinn::{self, crypto::rustls::QuicServerConfig};
use http::Response;
use rustls::pki_types::{self, CertificateDer, PrivateKeyDer};
use rustls::pki_types::pem::PemObject;

static ALPN: &[u8] = b"h3";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    rustls::crypto::aws_lc_rs::default_provider().install_default().unwrap();
    let cert = CertificateDer::from_pem_file("certs/server-cert.pem")?;
    let key = PrivateKeyDer::from_pem_file("certs/server-key.pem")?;

    let mut tls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)?;
    tls_config.alpn_protocols = vec![ALPN.into()];

    let server_config = quinn::ServerConfig::with_crypto(Arc::new(QuicServerConfig::try_from(tls_config)?));
    let endpoint = quinn::Endpoint::server(server_config, "127.0.0.1:4433".parse()?)?;

    println!("listening on 127.0.0.1:4433");

    while let Some(connecting) = endpoint.accept().await {
        tokio::spawn(async move {
            if let Ok(connection) = connecting.await {
                let mut h3_conn = h3::server::Connection::new(h3_quinn::Connection::new(connection))
                    .await
                    .unwrap();
                while let Ok(Some(resolver)) = h3_conn.accept().await {
                    tokio::spawn(async move {
                        let (request, mut stream) = resolver.resolve_request().await.unwrap();
                        println!("{} {}", request.method(), request.uri());
                        let response = Response::builder().status(200).body(()).unwrap();
                        stream.send_response(response).await.unwrap();
                        stream
                            .send_data(Bytes::from_static(b"Hello from h3"))
                            .await
                            .unwrap();
                        stream.finish().await.unwrap();
                    });
                }
            }
        });
    }

    endpoint.wait_idle().await;
    Ok(())
}

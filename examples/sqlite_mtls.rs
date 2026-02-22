//! Example: mTLS with a SQLite-backed certificate revocation verifier.
//!
//! Demonstrates how to use a custom `ClientCertVerifier` with `Server::new_with_mtls`.
//! The verifier delegates PKI chain validation to `WebPkiClientVerifier` and then
//! checks the certificate's SHA-256 fingerprint against a SQLite revocation table.
//! Certificates that pass PKI validation are accepted unless explicitly revoked.
//!
//! Usage:
//!   cargo run --example sqlite_mtls --features quic
//!
//! The example:
//! 1. Creates an in-memory SQLite database with a revoked certificate fingerprint
//! 2. Builds a custom `ClientCertVerifier` that rejects revoked certificates
//! 3. Starts a QUIC server using this verifier
//! 4. Connects a client whose certificate is NOT revoked (succeeds)
//! 5. Connects a client whose certificate IS revoked (rejected)

use std::fmt;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use echo_protocol::EchoChannel;
use jetstream::prelude::*;
use jetstream_macros::service;
use jetstream_quic::{
    Client, QuicRouter, QuicRouterHandler, QuicTransport, Server,
};

use rusqlite::Connection as SqliteConnection;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, UnixTime};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::server::WebPkiClientVerifier;
use rustls::{
    DigitallySignedStruct, DistinguishedName, Error, RootCertStore,
    SignatureScheme,
};

#[service]
pub trait Echo {
    async fn ping(&mut self) -> Result<()>;
}

#[derive(Clone)]
struct EchoImpl {}

impl Echo for EchoImpl {
    async fn ping(&mut self) -> Result<()> {
        eprintln!("Ping received");
        Ok(())
    }
}

/// A client certificate verifier that delegates cryptographic verification to
/// `WebPkiClientVerifier` and then checks the certificate's SHA-256 fingerprint
/// against a SQLite revocation table. Certificates that pass PKI validation are
/// accepted unless their fingerprint appears in the `revoked_certs` table.
struct SqliteRevocationVerifier {
    /// Inner verifier that handles PKI chain validation and signature checks.
    inner: Arc<dyn ClientCertVerifier>,
    /// SQLite database connection (wrapped in a Mutex for Send + Sync).
    db: std::sync::Mutex<SqliteConnection>,
}

impl SqliteRevocationVerifier {
    /// Create a new verifier.
    ///
    /// * `ca_cert` - CA certificate used to verify the client's certificate chain.
    /// * `db` - SQLite connection with a `revoked_certs` table containing
    ///          a `fingerprint TEXT` column of hex-encoded SHA-256 fingerprints.
    fn new(ca_cert: CertificateDer<'static>, db: SqliteConnection) -> Self {
        let mut root_store = RootCertStore::empty();
        root_store.add(ca_cert).expect("Failed to add CA cert");

        let inner = WebPkiClientVerifier::builder(Arc::new(root_store))
            .build()
            .expect("Failed to build inner WebPki verifier");

        Self {
            inner,
            db: std::sync::Mutex::new(db),
        }
    }

    /// Compute a hex-encoded SHA-256 fingerprint of a DER-encoded certificate.
    fn fingerprint(cert: &CertificateDer<'_>) -> String {
        sha256::digest(cert.as_ref())
    }
}

impl fmt::Debug for SqliteRevocationVerifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SqliteRevocationVerifier").finish()
    }
}

impl ClientCertVerifier for SqliteRevocationVerifier {
    fn offer_client_auth(&self) -> bool {
        true
    }

    fn client_auth_mandatory(&self) -> bool {
        true
    }

    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        self.inner.root_hint_subjects()
    }

    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        now: UnixTime,
    ) -> std::result::Result<ClientCertVerified, Error> {
        // First, do standard PKI chain validation.
        self.inner
            .verify_client_cert(end_entity, intermediates, now)?;

        // Then check the fingerprint against the revocation table.
        let fp = Self::fingerprint(end_entity);
        let db = self.db.lock().unwrap();
        let mut stmt = db
            .prepare_cached(
                "SELECT 1 FROM revoked_certs WHERE fingerprint = ?1",
            )
            .map_err(|e| {
                Error::General(format!("SQLite prepare error: {}", e))
            })?;

        let revoked: bool = stmt
            .query_row(rusqlite::params![fp], |_row| Ok(true))
            .unwrap_or(false);

        if revoked {
            eprintln!("Certificate REVOKED (fingerprint: {})", fp);
            Err(Error::General(format!(
                "client certificate has been revoked: {}",
                fp
            )))
        } else {
            eprintln!("Certificate accepted (fingerprint: {})", fp);
            Ok(ClientCertVerified::assertion())
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<
        rustls::client::danger::HandshakeSignatureValid,
        Error,
    > {
        self.inner.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<
        rustls::client::danger::HandshakeSignatureValid,
        Error,
    > {
        self.inner.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.inner.supported_verify_schemes()
    }
}

pub static CA_CERT_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/ca.pem");
pub static CLIENT_CERT_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/client.pem");
pub static CLIENT_KEY_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/client.key");
pub static CLIENT2_CERT_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/client2.pem");
pub static CLIENT2_KEY_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/client2.key");
pub static SERVER_CERT_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/server.pem");
pub static SERVER_KEY_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/server.key");

fn load_certs(path: &str) -> Vec<CertificateDer<'static>> {
    let data = std::fs::read(Path::new(path)).expect("Failed to read cert");
    rustls_pemfile::certs(&mut &*data)
        .filter_map(|r| r.ok())
        .collect()
}

fn load_key(path: &str) -> PrivateKeyDer<'static> {
    let data = std::fs::read(Path::new(path)).expect("Failed to read key");
    rustls_pemfile::private_key(&mut &*data)
        .expect("Failed to parse key")
        .expect("No key found")
}

async fn server(
    addr: SocketAddr,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server_cert = load_certs(SERVER_CERT_PEM).pop().unwrap();
    let server_key = load_key(SERVER_KEY_PEM);
    let ca_cert = load_certs(CA_CERT_PEM).pop().unwrap();
    let revoked_cert_der = load_certs(CLIENT2_CERT_PEM).pop().unwrap();

    // Create an in-memory SQLite database with a revocation table.
    let db = SqliteConnection::open_in_memory()
        .expect("Failed to open SQLite in-memory db");
    db.execute_batch(
        "CREATE TABLE revoked_certs (fingerprint TEXT NOT NULL UNIQUE);",
    )
    .expect("Failed to create table");

    // Revoke client2's certificate.
    let revoked_fp = SqliteRevocationVerifier::fingerprint(&revoked_cert_der);
    db.execute(
        "INSERT INTO revoked_certs (fingerprint) VALUES (?1)",
        rusqlite::params![revoked_fp],
    )
    .expect("Failed to insert fingerprint");
    eprintln!("Revoked certificate fingerprint: {}", revoked_fp);

    // Build the custom verifier
    let verifier = Arc::new(SqliteRevocationVerifier::new(ca_cert, db));

    let echo_service = echo_protocol::EchoService { inner: EchoImpl {} };

    let rpc_router = Arc::new(
        jetstream_rpc::Router::new()
            .with_handler(echo_protocol::PROTOCOL_NAME, echo_service),
    );
    let quic_handler = QuicRouterHandler::new(rpc_router);

    let mut router = QuicRouter::new();
    router.register(Arc::new(quic_handler));

    let server = Server::new_with_mtls(
        vec![server_cert],
        server_key,
        verifier,
        addr,
        router,
    );

    eprintln!("Server listening on {}", addr);
    server.run().await;

    Ok(())
}

/// Client whose certificate is NOT revoked — should succeed.
async fn allowed_client(
    addr: SocketAddr,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let ca_cert = load_certs(CA_CERT_PEM).pop().unwrap();
    let client_cert = load_certs(CLIENT_CERT_PEM).pop().unwrap();
    let client_key = load_key(CLIENT_KEY_PEM);

    let alpn = vec![b"jetstream".to_vec()];
    let bind_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
    let client = Client::new_with_mtls(
        ca_cert,
        client_cert,
        client_key,
        alpn,
        bind_addr,
    )?;

    let connection = client.connect(addr, "localhost").await?;

    let (send, recv) = connection.open_bi().await?;
    let transport: QuicTransport<EchoChannel> = (send, recv).into();
    let mut chan = EchoChannel::new(10, Box::new(transport));
    chan.negotiate_version(u32::MAX).await?;

    eprintln!("[allowed]   Sending ping...");
    chan.ping().await?;
    eprintln!(
        "[allowed]   Pong received - SQLite verifier accepted our certificate!"
    );

    Ok(())
}

/// Client whose certificate IS revoked — should be rejected.
/// Uses client2, which is signed by the same CA and has clientAuth EKU,
/// but whose fingerprint is in the SQLite revocation table.
async fn revoked_client(
    addr: SocketAddr,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let ca_cert = load_certs(CA_CERT_PEM).pop().unwrap();
    let unauthorized_cert = load_certs(CLIENT2_CERT_PEM).pop().unwrap();
    let unauthorized_key = load_key(CLIENT2_KEY_PEM);

    let alpn = vec![b"jetstream".to_vec()];
    let bind_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
    let client = Client::new_with_mtls(
        ca_cert,
        unauthorized_cert,
        unauthorized_key,
        alpn,
        bind_addr,
    )?;

    eprintln!("[revoked]   Connecting with revoked certificate...");
    match client.connect(addr, "localhost").await {
        Ok(connection) => {
            // In QUIC, the client handshake completes before the server
            // verifies the client certificate. Race every operation
            // against the connection being closed by the server.
            let try_ping = async {
                let (send, recv) = connection.open_bi().await?;
                let transport: QuicTransport<EchoChannel> = (send, recv).into();
                let mut chan = EchoChannel::new(10, Box::new(transport));
                chan.negotiate_version(u32::MAX).await?;
                chan.ping().await?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
            };
            tokio::select! {
                result = try_ping => {
                    match result {
                        Ok(_) => {
                            eprintln!("[revoked]   ERROR: ping succeeded — should have been rejected!");
                        }
                        Err(e) => {
                            eprintln!("[revoked]   Correctly rejected: {}", e);
                        }
                    }
                }
                reason = connection.closed() => {
                    eprintln!("[revoked]   Correctly rejected: {}", reason);
                }
            }
        }
        Err(e) => {
            eprintln!("[revoked]   Correctly rejected: {}", e);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .ok();

    let addr: SocketAddr = "127.0.0.1:4437".parse().unwrap();
    tokio::select! {
        _ = server(addr) => {},
        _ = async {
            // First: non-revoked client succeeds
            allowed_client(addr).await.unwrap();
            // Then: revoked client fails
            revoked_client(addr).await.unwrap();
        } => {},
    }
}

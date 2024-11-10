use criterion::{black_box, criterion_group, criterion_main, Criterion};
use echo_protocol::{Rframe, Tframe, Tmessage, Tping};
use jetstream::prelude::*;
use jetstream_client::DialQuic;
use jetstream_rpc::SharedJetStreamService;
use jetstream_server::{proxy::Proxy, quic_server::QuicServer};
use jetstream_wireformat::wire_format_extensions::AsyncWireFormatExt;
use okstd::prelude::*;
use s2n_quic::{provider::tls, Server};
use std::sync::OnceLock;
use std::{
    path::{self, Path},
    sync::Arc,
};
use tokio::{io::AsyncWriteExt, net::UnixListener, sync::Barrier};

static SOCKET_PATH: OnceLock<Box<Path>> = OnceLock::new();

// Certificate paths
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

#[service]
pub trait Echo: Send + Sync {
    async fn ping(&self, name: String) -> Result<String, std::io::Error>;
}

#[derive(Clone)]
struct EchoImpl;

impl Echo for EchoImpl {
    async fn ping(&self, name: String) -> Result<String, std::io::Error> {
        Ok(format!("Hello, {}!", name))
    }
}

// Add structure to hold our runtime state
struct RuntimeState {
    _tmp_dir: tmpdir::TmpDir, // Keep TmpDir alive
    socket_path: Box<Path>,
}

static STATE: OnceLock<RuntimeState> = OnceLock::new();

async fn setup_server_and_proxy() -> RuntimeState {
    // setup_logging(LevelFilter::Debug).expect("set logger");

    let tls = tls::default::Server::builder()
        .with_trusted_certificate(Path::new(CA_CERT_PEM))
        .unwrap()
        .with_certificate(Path::new(SERVER_CERT_PEM), Path::new(SERVER_KEY_PEM))
        .unwrap()
        .with_client_authentication()
        .unwrap()
        .build()
        .unwrap();

    let barrier = Arc::new(Barrier::new(3));
    let server_barrier = barrier.clone();

    tokio::spawn(async move {
        let server = Server::builder()
            .with_tls(tls)
            .unwrap()
            .with_io("127.0.0.1:4433")
            .unwrap()
            .start()
            .unwrap();

        let qsrv = QuicServer::new(SharedJetStreamService::new(
            echo_protocol::EchoProtocol::new(EchoImpl),
        ));
        server_barrier.wait().await;
        let _ = qsrv.serve(server).await;
    });

    let temp_dir = tmpdir::TmpDir::new("q9p").await.unwrap();
    let mut listen = temp_dir.to_path_buf();
    listen.push("q9p.sock");
    let listen_path = listen.into_boxed_path();
    let listener = UnixListener::bind(listen_path.clone()).unwrap();

    let cert = path::PathBuf::from(CLIENT_CERT_PEM).into_boxed_path();
    let key = path::PathBuf::from(CLIENT_KEY_PEM).into_boxed_path();
    let ca_cert = path::PathBuf::from(CA_CERT_PEM).into_boxed_path();

    let proxy_barrier = barrier.clone();
    tokio::spawn(async move {
        proxy_barrier.wait().await;
        let mut proxy = Proxy::new(
            DialQuic::new(
                "127.0.0.1".to_string(),
                4433,
                cert,
                key,
                ca_cert,
                "localhost".to_string(),
            ),
            listener,
        );
        let _ = proxy.run::<echo_protocol::EchoProtocol<EchoImpl>>().await;
    });

    barrier.wait().await;
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    RuntimeState {
        _tmp_dir: temp_dir,
        socket_path: listen_path,
    }
}

pub fn jetstream_benchmarks(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Initialize the global state
    let _ = STATE.get_or_init(|| rt.block_on(setup_server_and_proxy()));

    let mut group = c.benchmark_group("jetstream");
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(10));

    group.bench_function("ping_pong", |b| {
        b.to_async(&rt).iter(|| async {
            let socket_path = &STATE.get().unwrap().socket_path;
            let (mut read, mut write) =
                tokio::net::UnixStream::connect(socket_path)
                    .await
                    .unwrap()
                    .into_split();

            let test = Tframe {
                tag: 0,
                msg: Tmessage::Ping(Tping {
                    name: "world".to_string(),
                }),
            };

            match test.encode_async(&mut write).await {
                Ok(_) => {}
                Err(_) => return, // Handle potential write errors gracefully
            }
            if let Err(_) = write.flush().await {
                return;
            }

            match read.readable().await {
                Ok(_) => {}
                Err(_) => return,
            }

            match Rframe::decode_async(&mut read).await {
                Ok(resp) => assert_eq!(resp.tag, 0),
                Err(_) => return,
            }
        });
    });

    group.finish();
}

criterion_group!(benches, jetstream_benchmarks);
criterion_main!(benches);

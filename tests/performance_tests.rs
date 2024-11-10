use criterion::{criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;
use jetstream_client::{DialQuic, Connection};
use jetstream_server::quic_server::{start_server, QuicConfig};
use jetstream_rpc::{SharedJetStreamService, Service};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Barrier;

#[tokio::main]
async fn setup_server() {
    let server = jetstream_ufs::Ufs::new(PathBuf::from("/tmp"));
    let config = QuicConfig::default();
    start_server(server.get_handler(), config).await.unwrap();
}

fn performance_test_jetstream_client(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let cert = PathBuf::from("certs/client-cert.pem").into_boxed_path();
        let key = PathBuf::from("certs/client-key.pem").into_boxed_path();
        let ca_cert = PathBuf::from("certs/ca-cert.pem").into_boxed_path();
        let dial = DialQuic::new(
            "127.0.0.1".to_string(),
            4433,
            cert,
            key,
            ca_cert,
            "localhost".to_string(),
        );
        let mut connection = dial.dial().await.unwrap();
        let mut handle = connection.new_handle().await.unwrap();

        c.bench_function("jetstream_client_ping", |b| {
            b.iter(|| {
                let response = rt.block_on(handle.rpc("ping".to_string()));
                assert!(response.is_ok());
            });
        });
    });
}

fn performance_test_jetstream_server(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let barrier = Arc::new(Barrier::new(2));
        let server_barrier = barrier.clone();
        tokio::spawn(async move {
            setup_server().await;
            server_barrier.wait().await;
        });

        barrier.wait().await;

        let cert = PathBuf::from("certs/client-cert.pem").into_boxed_path();
        let key = PathBuf::from("certs/client-key.pem").into_boxed_path();
        let ca_cert = PathBuf::from("certs/ca-cert.pem").into_boxed_path();
        let dial = DialQuic::new(
            "127.0.0.1".to_string(),
            4433,
            cert,
            key,
            ca_cert,
            "localhost".to_string(),
        );
        let mut connection = dial.dial().await.unwrap();
        let mut handle = connection.new_handle().await.unwrap();

        c.bench_function("jetstream_server_ping", |b| {
            b.iter(|| {
                let response = rt.block_on(handle.rpc("ping".to_string()));
                assert!(response.is_ok());
            });
        });
    });
}

criterion_group!(
    benches,
    performance_test_jetstream_client,
    performance_test_jetstream_server
);
criterion_main!(benches);

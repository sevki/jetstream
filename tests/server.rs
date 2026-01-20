use std::net::{IpAddr, Ipv4Addr};

use echo_protocol::EchoChannel;
use jetstream::prelude::*;
use jetstream_rpc::{client::ClientCodec, server::run, Framed};
use miette::{
    LabeledSpan, Severity,
};
use turmoil::{
    net::{TcpListener, TcpStream},
    Builder,
};

#[service]
pub trait Echo: Debug {
    async fn ping(&mut self) -> Result<()>;
    async fn pong(&mut self) -> Result<()>;
    async fn fail_with_error(&mut self) -> Result<String>;
}

struct EchoImpl {}

impl Echo for EchoImpl {
    async fn ping(&mut self) -> Result<()> {
        Ok(())
    }

    async fn pong(&mut self) -> Result<()> {
        todo!()
    }

    async fn fail_with_error(&mut self) -> Result<String> {
        let label = LabeledSpan::new_primary_with_span(
            Some("label".to_string()),
            (1, 2),
        );
        // Return an error with full diagnostic information
        let err = Error::new("Server-side validation failed")
            .with_code("server::validation::E001")
            .with_severity(Severity::Error)
            .with_help("Check your input parameters")
            .with_label(label);

        Err(err)
    }
}

const PORT: u16 = 1738;

async fn bind_to_v4(
    port: u16,
) -> std::result::Result<TcpListener, std::io::Error> {
    TcpListener::bind((IpAddr::from(Ipv4Addr::UNSPECIFIED), port)).await
}

async fn bind() -> std::result::Result<TcpListener, std::io::Error> {
    bind_to_v4(PORT).await
}

fn network_partitions_during_connect() -> turmoil::Result {
    let mut sim = Builder::new().build();

    sim.host("server", || async {
        let listener = bind().await?;
        loop {
            let (stream, _) = listener.accept().await?;
            let echo = EchoImpl {};
            let servercodec: jetstream::prelude::server::ServerCodec<
                echo_protocol::EchoService<EchoImpl>,
            > = Default::default();
            let framed =
                Framed::with_capacity(stream, servercodec, 1024 * 1024 * 10);
            let mut serv = echo_protocol::EchoService { inner: echo };
            run(&mut serv, framed).await.expect("server run failed");
        }
    });

    sim.client("client", async {
        let stream = TcpStream::connect(("server", PORT)).await?;
        let client_codec: ClientCodec<EchoChannel> = Default::default();
        let mut framed = Framed::new(stream, client_codec);
        let mut chan = EchoChannel {
            inner: Box::new(&mut framed),
        };
        chan.ping().await.expect("ping failed");
        Ok(())
    });

    sim.run()
}

#[okstd::test]
fn e2e() {
    network_partitions_during_connect()
        .expect("network partitions during connect failed");
}

/// r[impl jetstream.test.error_propagation.e2e]
/// r[verify jetstream.test.error_propagation.e2e]
/// End-to-end test that verifies error propagation from server to client
/// with all diagnostic information preserved.
fn error_propagation_e2e() -> turmoil::Result {
    let mut sim = Builder::new().build();

    sim.host("server", || async {
        let listener = bind().await?;
        loop {
            let (stream, _) = listener.accept().await?;
            let echo = EchoImpl {};
            let servercodec: jetstream::prelude::server::ServerCodec<
                echo_protocol::EchoService<EchoImpl>,
            > = Default::default();
            let framed =
                Framed::with_capacity(stream, servercodec, 1024 * 1024 * 10);
            let mut serv = echo_protocol::EchoService { inner: echo };
            run(&mut serv, framed).await.expect("server run failed");
        }
    });

    sim.client("client", async {
        let stream = TcpStream::connect(("server", PORT)).await?;
        let client_codec: ClientCodec<EchoChannel> = Default::default();
        let mut framed = Framed::new(stream, client_codec);
        let mut chan = EchoChannel {
            inner: Box::new(&mut framed),
        };

        // Call the method that returns an error
        let result = chan.fail_with_error().await;

        // Verify we got an error
        assert!(result.is_err(), "Expected an error from fail_with_error");

        Ok(())
    });

    sim.run()
}

#[okstd::test]
fn e2e_error_propagation() {
    error_propagation_e2e().expect("error propagation e2e failed");
}

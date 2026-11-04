//! Versioned services, end to end.
//!
//! The unit tests in `jetstream_macros` assert what the expansion looks
//! like. This one asserts that it *works*: that both snapshots compile,
//! that one concrete type can serve both, that a client of each version
//! reaches the right method over a real (simulated) socket, and that
//! adding a version left the older one's wire ids where they were.

use std::net::{IpAddr, Ipv4Addr};

use jetstream::prelude::*;
use jetstream_rpc::{client::ClientCodec, server::run, Framed};
use turmoil::{
    net::{TcpListener, TcpStream},
    Builder,
};

/// One trait describing two versions of the same API.
///
/// `ping` is replaced at 1.0.0 — it gains an argument and a return
/// value. `pong` is untouched and carries forward. `shout` does not
/// exist before 1.0.0.
#[service]
pub trait Echo {
    #[since("0.1.0")]
    async fn ping(&self) -> Result<()>;

    #[since("0.1.0")]
    async fn pong(&self) -> Result<()>;

    #[since("1.0.0")]
    async fn ping(&self, msg: String) -> Result<String>;

    #[since("1.0.0")]
    async fn shout(&self, msg: String) -> Result<String>;
}

/// One server type implementing every version it speaks.
#[derive(Clone, Debug, Default)]
struct EchoImpl;

impl EchoV0_1_0 for EchoImpl {
    async fn ping(&self) -> Result<()> {
        Ok(())
    }

    async fn pong(&self) -> Result<()> {
        Ok(())
    }
}

impl EchoV1_0_0 for EchoImpl {
    async fn ping(&self, msg: String) -> Result<String> {
        Ok(format!("v1:{msg}"))
    }

    async fn pong(&self) -> Result<()> {
        Ok(())
    }

    async fn shout(&self, msg: String) -> Result<String> {
        Ok(msg.to_uppercase())
    }
}

const V0_PORT: u16 = 1742;
const V1_PORT: u16 = 1743;

async fn bind(port: u16) -> std::result::Result<TcpListener, std::io::Error> {
    TcpListener::bind((IpAddr::from(Ipv4Addr::UNSPECIFIED), port)).await
}

/// Both versions are served by the same type, on separate lanes, and a
/// client of each reaches its own methods.
fn both_versions_serve_independently() -> turmoil::Result {
    let mut sim = Builder::new().build();

    sim.host("v0", || async {
        let listener = bind(V0_PORT).await?;
        loop {
            let (stream, _) = listener.accept().await?;
            let codec: jetstream::prelude::server::ServerCodec<
                echov0_1_0_protocol::EchoV0_1_0Service<EchoImpl>,
            > = Default::default();
            let framed = Framed::with_capacity(stream, codec, 1024 * 1024);
            let mut serv =
                echov0_1_0_protocol::EchoV0_1_0Service { inner: EchoImpl };
            run(&mut serv, framed).await.expect("v0 server run failed");
        }
    });

    sim.host("v1", || async {
        let listener = bind(V1_PORT).await?;
        loop {
            let (stream, _) = listener.accept().await?;
            let codec: jetstream::prelude::server::ServerCodec<
                echov1_0_0_protocol::EchoV1_0_0Service<EchoImpl>,
            > = Default::default();
            let framed = Framed::with_capacity(stream, codec, 1024 * 1024);
            let mut serv =
                echov1_0_0_protocol::EchoV1_0_0Service { inner: EchoImpl };
            run(&mut serv, framed).await.expect("v1 server run failed");
        }
    });

    sim.client("client", async {
        // 0.1.0: `ping` takes nothing and answers with nothing.
        let stream = TcpStream::connect(("v0", V0_PORT)).await?;
        let codec: ClientCodec<echov0_1_0_protocol::EchoV0_1_0Channel> =
            Default::default();
        let framed = Framed::new(stream, codec);
        let v0 =
            echov0_1_0_protocol::EchoV0_1_0Channel::new(10, Box::new(framed));
        v0.ping().await.expect("v0 ping failed");
        v0.pong().await.expect("v0 pong failed");

        // 1.0.0: the replacement `ping` takes a message and answers with
        // one, and `shout` exists only here.
        let stream = TcpStream::connect(("v1", V1_PORT)).await?;
        let codec: ClientCodec<echov1_0_0_protocol::EchoV1_0_0Channel> =
            Default::default();
        let framed = Framed::new(stream, codec);
        let v1 =
            echov1_0_0_protocol::EchoV1_0_0Channel::new(10, Box::new(framed));
        assert_eq!(
            v1.ping("hello".to_string()).await.expect("v1 ping failed"),
            "v1:hello",
        );
        assert_eq!(
            v1.shout("hello".to_string()).await.expect("shout failed"),
            "HELLO",
        );
        v1.pong().await.expect("v1 pong failed");

        Ok(())
    });

    sim.run()
}

#[test]
fn versions_dispatch_independently() {
    both_versions_serve_independently()
        .expect("versioned services should serve independently");
}

/// One value, both traits. The method names collide, so every call has
/// to name the trait it means.
#[tokio::test]
async fn one_type_serves_both_versions_through_qualified_calls() {
    let echo = EchoImpl;

    EchoV0_1_0::ping(&echo).await.expect("0.1.0 ping");
    assert_eq!(
        EchoV1_0_0::ping(&echo, "x".to_string())
            .await
            .expect("1.0.0 ping"),
        "v1:x",
    );

    // `pong` is the same declaration in both snapshots, but it is still
    // two distinct trait methods and still needs qualifying.
    EchoV0_1_0::pong(&echo).await.expect("0.1.0 pong");
    EchoV1_0_0::pong(&echo).await.expect("1.0.0 pong");
}

/// r[verify jetstream.subscription.compat]
/// Adding 1.0.0 must not have moved 0.1.0's methods on the wire, and
/// `pong` must sit at the same id in both.
#[test]
fn wire_ids_are_stable_across_versions() {
    // `ping` was introduced first, `pong` second; ids are
    // `MESSAGE_ID_START + 2 * slot`, starting at 102.
    assert_eq!(echov0_1_0_protocol::TPING, 102);
    assert_eq!(echov0_1_0_protocol::RPING, 103);
    assert_eq!(echov0_1_0_protocol::TPONG, 104);
    assert_eq!(echov0_1_0_protocol::RPONG, 105);

    // 1.0.0 replaces `ping` in place and appends `shout`; nothing that
    // already existed moved.
    assert_eq!(echov1_0_0_protocol::TPING, 102);
    assert_eq!(echov1_0_0_protocol::RPING, 103);
    assert_eq!(echov1_0_0_protocol::TPONG, 104);
    assert_eq!(echov1_0_0_protocol::RPONG, 105);
    assert_eq!(echov1_0_0_protocol::TSHOUT, 106);
    assert_eq!(echov1_0_0_protocol::RSHOUT, 107);
}

/// Each version negotiates under its own name, which is what stops one
/// version's client decoding another version's payloads: `Server::version`
/// rejects a mismatched `Protocol::NAME` during `Tversion`.
#[test]
fn versions_are_distinct_protocols() {
    assert_eq!(echov0_1_0_protocol::PROTOCOL_NAME, "echov0_1_0");
    assert_eq!(echov1_0_0_protocol::PROTOCOL_NAME, "echov1_0_0");
    assert_ne!(
        echov0_1_0_protocol::PROTOCOL_NAME,
        echov1_0_0_protocol::PROTOCOL_NAME,
    );
    assert_ne!(
        echov0_1_0_protocol::PROTOCOL_VERSION,
        echov1_0_0_protocol::PROTOCOL_VERSION,
    );
}

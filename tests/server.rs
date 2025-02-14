use {
    echo_protocol::EchoChannel,
    jetstream::prelude::*,
    jetstream_rpc::Framed,
    server::service::run,
    std::net::{IpAddr, Ipv4Addr},
    turmoil::{
        net::{TcpListener, TcpStream},
        Builder,
    },
};

#[service]
pub trait Echo {
    async fn ping(&mut self) -> Result<(), Error>;
    async fn pong(&mut self) -> Result<(), Error>;
}

struct EchoImpl {}

impl Echo for EchoImpl {
    async fn ping(&mut self) -> Result<(), Error> {
        Ok(())
    }

    async fn pong(&mut self) -> Result<(), Error> {
        todo!()
    }
}

const PORT: u16 = 1738;

async fn bind_to_v4(port: u16) -> std::result::Result<TcpListener, std::io::Error> {
    TcpListener::bind((IpAddr::from(Ipv4Addr::UNSPECIFIED), port)).await
}

async fn bind() -> std::result::Result<TcpListener, std::io::Error> {
    bind_to_v4(PORT).await
}

fn network_partitions_during_connect() -> turmoil::Result {
    let mut sim = Builder::new().build();

    sim.host("server", || {
        async {
            let listener = bind().await?;
            loop {
                let (stream, _) = listener.accept().await?;
                let echo = EchoImpl {};
                let servercodec: jetstream::prelude::server::service::ServerCodec<
                    echo_protocol::EchoService<EchoImpl>,
                > = Default::default();
                let framed = Framed::with_capacity(stream, servercodec, 1024 * 1024 * 10);
                let mut serv = echo_protocol::EchoService { inner: echo };
                run(&mut serv, framed).await.expect("server run failed");
            }
        }
    });

    sim.client("client", async {
        let stream = TcpStream::connect(("server", PORT)).await?;
        let client_codec: jetstream_client::ClientCodec<EchoChannel> = Default::default();
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
    network_partitions_during_connect().expect("network partitions during connect failed");
}

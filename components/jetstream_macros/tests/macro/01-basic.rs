use {
    echo_protocol::EchoChannel,
    jetstream::prelude::*,
    okstd::prelude::*,
    server::service::run,
    std::net::{IpAddr, Ipv4Addr},
    turmoil::{
        net::{TcpListener, TcpStream},
        Builder,
    },
};

#[service]
pub trait Echo {
    async fn ping(&mut self, msg: String) -> Result<String, Error>;
}

struct EchoImpl {}

impl Echo for EchoImpl {
    async fn ping(&mut self, msg: String) -> Result<String, Error> {
        Ok(msg)
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
                let framed = Framed::new(stream, servercodec);
                let mut serv = echo_protocol::EchoService { inner: echo };
                run(&mut serv, framed).await.expect("run");
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
        chan.ping("ping".to_string()).await?;
        Ok(())
    });

    sim.run()
}

#[okstd::main]
async fn main() {
    network_partitions_during_connect().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[okstd::test]
    #[okstd::log(debug)]
    fn test_network_partitions_during_connect() {
        network_partitions_during_connect().unwrap()
    }
}

use {okstd::prelude::*, std::path::PathBuf, tokio::net::UnixListener, tokio_util::codec::Framed};

#[derive(FromArgs)]
/// JetStream Ufs Server
struct Ufs {
    /// root directory
    #[argh(option)]
    root: PathBuf,
    /// unix socket
    #[argh(option)]
    socket: PathBuf,
}

#[okstd::main]
async fn main() {
    let args: Ufs = argh::from_env();
    let unix_listener = UnixListener::bind(&args.socket).unwrap();
    while let Ok((stream, _)) = unix_listener.accept().await {
        let mut service = jetstream_ufs::Server::new(
            args.root.clone(),
            std::collections::BTreeMap::new(),
            std::collections::BTreeMap::new(),
        )
        .unwrap();
        let servercodec: jetstream_server::service::ServerCodec<jetstream_ufs::Server> =
            Default::default();

        let service_transport = Framed::new(stream, servercodec);
        jetstream_server::service::run(&mut service, service_transport)
            .await
            .unwrap()
    }
}

use jetstream_server::quic_server::start_server;
use okstd::prelude::*;
use std::path::PathBuf;

#[derive(FromArgs)]
/// JetStream Ufs Server
struct Ufs {
    /// root directory
    #[argh(option)]
    root: PathBuf,
}

#[okstd::main]
async fn main() {
    let args: Ufs = argh::from_env();

    let server = jetstream_ufs::Ufs::new(args.root);
    start_server(server.get_handler()).await
}

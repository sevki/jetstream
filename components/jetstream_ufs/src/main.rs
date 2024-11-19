use jetstream_server::quic::{start_server, QuicConfig};
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
    let config = QuicConfig::default();
    start_server(server.get_handler(), config).await
}

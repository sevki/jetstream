//! The `jetstream_ufs` server binary.
//!
//! Linux only, for the reasons in the crate documentation. The binary
//! still builds off Linux — a bin target with no `main` does not link —
//! but says why it cannot run rather than failing mysteriously.

#![cfg_attr(not(target_os = "linux"), allow(unused_imports))]

#[cfg(target_os = "linux")]
use std::path::PathBuf;

#[cfg(target_os = "linux")]
use argh::FromArgs;
#[cfg(target_os = "linux")]
use jetstream_rpc::server::{run, ServerCodec};
#[cfg(target_os = "linux")]
use tokio::net::UnixListener;
#[cfg(target_os = "linux")]
use tokio_util::codec::Framed;

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
#[tokio::main]
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
        let server_codec: ServerCodec<jetstream_ufs::Server> =
            Default::default();

        let service_transport = Framed::new(stream, server_codec);
        run(&mut service, service_transport).await.unwrap()
    }
}

/// Off Linux there is no server to run, and saying so beats a linker
/// error or a binary that is silently absent.
#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!(
        "jetstream_ufs serves 9P from a local filesystem using Linux's \
         getdents64; it is not available on this platform.",
    );
    std::process::exit(1);
}

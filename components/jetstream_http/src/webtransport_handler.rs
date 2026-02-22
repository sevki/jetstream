use std::sync::Arc;

use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use h3::quic;
use h3_webtransport::server::WebTransportSession;
use jetstream_rpc::{
    server::{Server, ServerCodec},
    Error, Frame, Router as RpcRouter,
};
use tokio_util::codec::{FramedRead, FramedWrite};

// r[impl jetstream.webtransport.upstream-initiated]
/// Opens a server-initiated bidirectional stream to the downstream client.
///
/// This allows the server to push RPC frames to the client over a new bidi stream.
/// The returned writer/reader pair uses `ServerCodec` for frame encoding/decoding.
///
/// # r[impl jetstream.webtransport.upstream-initiated.use-case]
/// Use this for server-push scenarios where the server needs to initiate
/// communication with the client (e.g., pushing notifications, streaming updates).
pub async fn open_upstream_bidi<T>(
    session: &WebTransportSession<h3_quinn::Connection, Bytes>,
    frame: Frame<T::Response>,
) -> Result<Frame<T::Request>, Error>
where
    T: Server<Error = Error> + Send + Sync + Clone + 'static,
{
    let session_id = session.session_id();
    let stream = session
        .open_bi(session_id)
        .await
        .map_err(|e| Error::from(std::io::Error::other(e.to_string())))?;
    let (send, recv) = quic::BidiStream::split(stream);
    let mut writer = FramedWrite::new(send, ServerCodec::<T>::new());
    let mut reader = FramedRead::new(recv, ServerCodec::<T>::new());

    writer.send(frame).await?;

    match reader.next().await {
        Some(Ok(response)) => Ok(response),
        Some(Err(e)) => Err(e),
        None => Err(Error::from(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "upstream bidi stream closed before response",
        ))),
    }
}

// r[impl jetstream.webtransport.bidi]
// r[impl jetstream.webtransport.bidi.concurrent]
// r[impl jetstream.webtransport.bidi.multi-stream]
// r[impl jetstream.webtransport.lifecycle]
// r[impl jetstream.webtransport.errors]
// r[impl jetstream.webtransport.errors.session]
// r[impl jetstream.webtransport.router]
// r[impl jetstream.webtransport.router.per-stream-version]
/// A WebTransport handler that uses an `RpcRouter` for per-stream
/// version-based protocol dispatch. Each bidi stream performs its own
/// Tversion/Rversion negotiation and is routed to the appropriate handler.
pub struct RouterHandler(pub Arc<RpcRouter>);

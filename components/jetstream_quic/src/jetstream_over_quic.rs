use crate::QuicHandler;
use async_trait::async_trait;
use jetstream_rpc::{context::Context, Router};
use quinn::Connection;
use std::sync::Arc;

/// A QUIC protocol handler that uses an `RpcRouter` for per-stream
/// version-based protocol dispatch.
pub struct QuicRouterHandler {
    pub router: Arc<Router>,
}

impl QuicRouterHandler {
    pub fn new(router: Arc<Router>) -> Self {
        Self { router }
    }
}

const ALPN: &str = "jetstream";

#[async_trait]
impl QuicHandler for QuicRouterHandler {
    fn alpns(&self) -> Vec<String> {
        vec![ALPN.to_string()]
    }

    async fn accept(&self, ctx: Context, conn: Connection) {
        let router = self.router.clone();
        while let Ok((send, recv)) = conn.accept_bi().await {
            let router = router.clone();
            let ctx = ctx.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    router.accept(ctx, Box::new(recv), Box::new(send)).await
                {
                    eprintln!("Router dispatch error on QUIC stream: {}", e);
                }
            });
        }
    }
}

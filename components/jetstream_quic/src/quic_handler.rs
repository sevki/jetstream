use jetstream_rpc::context::Context;
use quinn::Connection;

#[async_trait::async_trait]
pub trait ProtocolHandler: Send + Sync {
    fn alpn(&self) -> String;
    async fn accept(&self, ctx: Context, conn: Connection) -> ();
}

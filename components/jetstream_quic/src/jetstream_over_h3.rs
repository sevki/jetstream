use async_trait::async_trait;
use jetstream_rpc::server::Server;

use crate::HttpRequestHandler;

#[async_trait]
impl<P: Server + 'static> HttpRequestHandler<P::Request, P::Response> for P {
    async fn handle_request(
        &self,
        _ctx: jetstream_rpc::context::Context,
        _req: http::Request<P::Request>,
    ) -> http::Response<P::Response> {
        todo!()
    }
}

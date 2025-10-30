use jetstream::cloudflare::DefaultHtmlFallback;
use worker::{event, *};

use crate::{radar_protocol, Radar};
struct RadarWorker;

impl Radar for RadarWorker {
    async fn ping(
        &mut self,
    ) -> std::result::Result<(), jetstream::prelude::Error> {
        Ok(())
    }
}

#[event(fetch)]
async fn fetch(
    req: Request,
    env: Env,
    ctx: worker::Context,
) -> Result<Response> {
    let handler = radar_protocol::RadarService { inner: RadarWorker };
    let mut router =
        jetstream::cloudflare::Router::<DefaultHtmlFallback>::new([handler]);
    router.fetch(req, env, ctx).await
}

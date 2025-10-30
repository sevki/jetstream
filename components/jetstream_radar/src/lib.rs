use jetstream::prelude::*;

#[cfg(feature = "server")]
mod server;

#[service]
pub trait Radar {
    async fn ping(
        &mut self,
    ) -> std::result::Result<(), jetstream::prelude::Error>;
}

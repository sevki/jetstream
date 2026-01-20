use jetstream::prelude::*;

#[cfg(feature = "server")]
mod server;

#[service]
pub trait Radar {
    async fn ping(&mut self) -> Result<()>;
}

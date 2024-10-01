use jetstream::JetStreamProtocol;
use radar::{
    Radar, RadarProtocol, RadarService, Rframe, Rmessage, Rping, Rversion,
    Tframe, Tmessage, Tping, Tversion, Version, PROTOCOL_VERSION,
};
pub use tokio::io::{AsyncRead, AsyncWrite};

#[jetstream::protocol]
mod radar {
    use std::fmt::Debug;
    use slog_scope::debug;

    #[derive(Debug, JetStreamWireFormat)]
    pub struct Version {
        pub msize: u32,
        pub version: String,
    }
    #[async_trait::async_trait]
    pub trait Radar {
        async fn version(&mut self, req: Version) -> Version;
        fn ping(&mut self) -> ();
    }
}

struct MyRadar;

impl Radar for MyRadar {
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn version<'life0, 'async_trait>(
        &'life0 mut self,
        req: Version,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Version>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move { req })
    }

    fn ping(&mut self) -> () {
        ()
    }
}

use jetstream::service::JetStreamAsyncService;

fn main() {
    let mut r = MyRadar;
    let mut server = radar::RadarServer::new(r);
    
    futures::executor::block_on(async {
        let ver = server
            .rpc(Tframe {
                tag: 0,
                msg: Tmessage::Version(Tversion {
                    tag: 0,
                    req: Version {
                        msize: 0,
                        version: PROTOCOL_VERSION.to_string(),
                    },
                }),
            })
            .await.unwrap();

        match ver.msg {
            Rmessage::Version(v) => {
                println!("Version: {:?}", v.1.version);
            }
            _ => {
                println!("Unexpected response");
            }
        }
    });
}

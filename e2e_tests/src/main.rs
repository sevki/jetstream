use radar::{
    Radar, RadarProtocol, Rmessage, Tframe, Tmessage, Tversion, Version,
    PROTOCOL_VERSION,
};
pub use tokio::io::{AsyncRead, AsyncWrite};

#[jetstream::service]
mod radar {

    #[derive(JetStreamWireFormat)]
    pub struct Version {
        pub msize: u32,
        pub version: String,
    }
    #[async_trait::async_trait]
    pub trait Radar {
        async fn version(&mut self, req: Version) -> Version;
        fn ping(&mut self) -> u8;
    }
}
struct MyRadar;

impl Radar for MyRadar {
    fn ping(&mut self) -> u8 {
        0
    }

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
}

impl RadarProtocol for MyRadar {}

fn main() {
    let mut r = MyRadar;
    futures::executor::block_on(async {
        let ver = r
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
            .await;

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

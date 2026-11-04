use jetstream_macros::service;

#[service]
pub trait Echo {
    #[since("1.0.0")]
    #[until("0.9.0")]
    async fn ping(&self) -> Result<(), std::io::Error>;
}

fn main() {}

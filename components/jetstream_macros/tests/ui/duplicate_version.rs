use jetstream_macros::service;

#[service]
pub trait Echo {
    #[since("0.1.0")]
    async fn ping(&self) -> Result<(), std::io::Error>;

    #[since("0.1.0")]
    async fn ping(&self, msg: String) -> Result<String, std::io::Error>;
}

fn main() {}

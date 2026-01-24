#[cfg(channel)]
mod channel;
#[cfg(notify)]
mod notify;
#[cfg(semaphor)]
mod semaphor;

#[cfg(channel)]
pub use channel::TagPool;
#[cfg(notify)]
pub use notify::TagPool;
#[cfg(semaphor)]
pub use semaphor::TagPool;

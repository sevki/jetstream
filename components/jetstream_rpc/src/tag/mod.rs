#[cfg(channel)]
mod channel;
#[cfg(notify)]
mod notify;
#[cfg(semaphor)]
mod semaphor;

#[cfg(channel)]
use channel::TagPool as Backend;
#[cfg(notify)]
use notify::TagPool as Backend;
#[cfg(semaphor)]
use semaphor::TagPool as Backend;

/// r[impl jetstream.subscription.cancel]
/// Three regions of the tag space, not one pool.
///
/// A subscription holds its tag for its whole life. Drawing
/// subscriptions and unary calls from one pool therefore means enough
/// live subscriptions make an ordinary call impossible — at
/// `max_concurrent_requests = 1`, one open room blocks every `post` for
/// its duration.
///
/// Cancellation is worse than starved, it deadlocks: a cancellation
/// needs a **fresh** tag, the subscription's own tag is not released
/// until its terminator arrives, and the terminator cannot arrive until
/// the cancellation is sent. The pool is saturated by exactly the
/// subscriptions the cancellation exists to end.
///
/// `r[jetstream.subscription.cancel]` requires control capacity reserved
/// outside the pool and forbids subscriptions exhausting the capacity
/// available to ordinary calls. Both are satisfied here by giving each
/// its own region: ordinary calls keep `1..=size`, and the meaning
/// `max_concurrent_requests` always had, while subscriptions and
/// cancellations are drawn from above it.
///
/// The control region is sized equal to the streaming region, which is
/// what makes the deadlock unreachable rather than merely unlikely: at
/// most `span` subscriptions can exist, so at most `span` cancellations
/// can ever be in flight at once.
///
/// The backend is whichever of `channel`, `notify` or `semaphor`
/// `JETSTREAM_TAG_POOL_BACKEND` selected; each region is one instance of
/// it, and the offsets live here so no backend has to know about any of
/// this.
pub struct TagPool {
    ordinary: Backend,
    streaming: Backend,
    control: Backend,
    /// First tag of the streaming region; also one past the ordinary one.
    streaming_base: u16,
    /// First tag of the control region.
    control_base: u16,
}

impl TagPool {
    pub fn new(size: u16) -> Self {
        let rest = u16::MAX - size;
        let span = rest / 2;
        TagPool {
            ordinary: Backend::new(size),
            streaming: Backend::new(span),
            control: Backend::new(rest - span),
            streaming_base: size,
            control_base: size.saturating_add(span),
        }
    }

    /// A tag for a unary call. Bounded by `max_concurrent_requests`, and
    /// unaffected by how many subscriptions are open.
    pub async fn acquire_tag(&self) -> u16 {
        self.ordinary.acquire_tag().await
    }

    /// r[impl jetstream.subscription.cancel]
    /// A tag for a subscription, from its own region, so no number of
    /// live subscriptions can make an ordinary call impossible.
    pub async fn acquire_streaming_tag(&self) -> u16 {
        self.streaming_base + self.streaming.acquire_tag().await
    }

    /// r[impl jetstream.subscription.cancel]
    /// A tag for a cancellation, from capacity reserved outside both
    /// other regions.
    pub async fn acquire_control_tag(&self) -> u16 {
        self.control_base + self.control.acquire_tag().await
    }

    pub async fn release_tag(&self, tag: u16) {
        if tag > self.control_base {
            self.control.release_tag(tag - self.control_base).await
        } else if tag > self.streaming_base {
            self.streaming.release_tag(tag - self.streaming_base).await
        } else {
            self.ordinary.release_tag(tag).await
        }
    }

    /// Which region a tag came from. Used by the multiplexer to tell a
    /// subscription's tag from an ordinary call's without consulting the
    /// in-flight map.
    pub fn is_streaming_tag(&self, tag: u16) -> bool {
        tag > self.streaming_base && tag <= self.control_base
    }
}

#[cfg(test)]
mod tests;

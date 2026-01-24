use std::sync::{
    atomic::{AtomicU16, Ordering},
    Arc,
};
use tokio::sync::{Mutex, Semaphore};

pub struct TagPool {
    pub(crate) next_tag: AtomicU16,
    pub(crate) semaphore: Arc<Semaphore>,
    pub(crate) freed: Arc<Mutex<Vec<u16>>>,
    pub(crate) size: u16,
}

/// https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html#rate-limiting-using-a-token-bucket
impl TagPool {
    pub fn new(size: u16) -> Self {
        Self {
            next_tag: AtomicU16::new(1),
            semaphore: Arc::new(Semaphore::new(size as usize)),
            freed: Arc::new(Mutex::new(Vec::new())),
            size,
        }
    }

    pub async fn acquire_tag(&self) -> u16 {
        // Try fresh tag first
        let tag = self.next_tag.fetch_add(1, Ordering::Relaxed);
        if tag <= self.size {
            return tag;
        }
        // This can return an error if the semaphore is closed, but we
        // never close it, so this error can never happen.
        let permit = self.semaphore.acquire().await.unwrap();
        // To avoid releasing the permit back to the semaphore, we use
        // the `SemaphorePermit::forget` method.
        permit.forget();

        // Otherwise recycle
        self.freed
            .lock()
            .await
            .pop()
            .expect("semaphore guarantees availability")
    }

    pub async fn release_tag(&self, tag: u16) {
        self.freed.lock().await.push(tag);
        self.semaphore.add_permits(1);
    }
}

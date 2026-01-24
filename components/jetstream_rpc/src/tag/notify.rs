use std::sync::atomic::{AtomicU16, Ordering};
use tokio::sync::{Mutex, Notify};

pub struct TagPool {
    next_tag: AtomicU16,
    freed: Mutex<Vec<u16>>,
    notify: Notify,
    size: u16,
}

impl TagPool {
    pub fn new(size: u16) -> Self {
        Self {
            next_tag: AtomicU16::new(1),
            freed: Mutex::new(Vec::new()),
            notify: Notify::new(),
            size,
        }
    }

    pub async fn acquire_tag(&self) -> u16 {
        // Try fresh tag first
        let tag = self.next_tag.fetch_add(1, Ordering::Relaxed);
        if tag <= self.size {
            return tag;
        }

        // Wait for recycled tag
        loop {
            // Try to pop a freed tag
            {
                let mut freed = self.freed.lock().await;
                if let Some(tag) = freed.pop() {
                    return tag;
                }
            }
            // No tag available, wait for notification
            self.notify.notified().await;
        }
    }

    pub async fn release_tag(&self, tag: u16) {
        self.freed.lock().await.push(tag);
        self.notify.notify_one();
    }
}

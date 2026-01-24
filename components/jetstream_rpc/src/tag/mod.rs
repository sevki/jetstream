use std::sync::{
    atomic::{AtomicU16, Ordering},
    Arc,
};
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct TagPool {
    next_tag: Arc<AtomicU16>,
    size: u16,
    recycled: Arc<tokio::sync::Mutex<mpsc::Receiver<u16>>>,
    recycle_tx: mpsc::Sender<u16>,
}

impl TagPool {
    pub fn new(size: u16) -> Self {
        let (tx, rx) = mpsc::channel(size as usize);
        Self {
            next_tag: Arc::new(AtomicU16::new(1)),
            size,
            recycled: Arc::new(tokio::sync::Mutex::new(rx)),
            recycle_tx: tx,
        }
    }

    pub async fn acquire_tag(&self) -> u16 {
        // Try fresh first
        let tag = self.next_tag.fetch_add(1, Ordering::Relaxed);
        if tag <= self.size {
            return tag;
        }

        // Wait for recycled
        self.recycled
            .lock()
            .await
            .recv()
            .await
            .expect("pool not dropped")
    }

    pub async fn release_tag(&self, tag: u16) {
        self.recycle_tx.send(tag).await.ok();
    }
}

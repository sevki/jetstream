use std::sync::{self, Arc, Mutex};

use jetstream_error::Result;

#[derive(Clone)]
pub struct TagPool {
    next_tag: Arc<sync::atomic::AtomicU16>,
    freed: Arc<Mutex<Vec<u16>>>,
    exhausted: bool,
}

impl Default for TagPool {
    fn default() -> Self {
        Self {
            next_tag: Arc::new(sync::atomic::AtomicU16::new(0)),
            freed: Arc::new(Mutex::new(Vec::new())),
            exhausted: false,
        }
    }
}

impl<'a> TagPool {
    pub fn get_tag(&'a mut self) -> Result<u16> {
        // first check if we have a released slot.
        let mut free = self.freed.lock().unwrap();

        if let Some(tag) = free.pop() {
            Ok(tag)
        } else if !self.exhausted {
            let tag =
                self.next_tag.fetch_add(1, sync::atomic::Ordering::Relaxed);
            if tag == u16::MAX {
                self.exhausted = true;
            }
            Ok(tag)
        } else {
            Err(jetstream_error::Error::new("too many requests inflight"))
        }
    }

    pub(crate) fn release_tag(&mut self, tag: u16) {
        let mut free = self.freed.lock().unwrap();
        free.push(tag);
    }
}

#[cfg(test)]
mod tests;

#![cfg(loom)]
use super::*;
use loom::sync::atomic::AtomicUsize;
use loom::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use loom::sync::Arc;
use loom::thread;

#[test]
fn test_map_acquire() {
    loom::model(|| {});
}

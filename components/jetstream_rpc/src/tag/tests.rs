use super::TagPool;

/// r[verify jetstream.subscription.cancel]
/// The starvation case, stated in the specification and previously
/// reachable: one subscription at `max_concurrent_requests = 1` used to
/// take the only tag there was, and every later unary call blocked for
/// as long as the subscription lived.
#[tokio::test]
async fn a_subscription_cannot_starve_ordinary_calls() {
    let pool = TagPool::new(1);
    let _subscription = pool.acquire_streaming_tag().await;
    let unary = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        pool.acquire_tag(),
    )
    .await
    .expect("an ordinary call must still get a tag");
    assert_eq!(unary, 1, "ordinary calls keep the range they always had");
}

/// r[verify jetstream.subscription.cancel]
/// The deadlock: a cancellation needs a fresh tag, and the tag it would
/// free is not released until the terminator its own delivery produces.
/// Control capacity is reserved outside the pool for exactly this.
#[tokio::test]
async fn cancellation_has_capacity_when_everything_else_is_saturated() {
    let pool = TagPool::new(1);
    let _unary = pool.acquire_tag().await;
    let _subscription = pool.acquire_streaming_tag().await;
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        pool.acquire_control_tag(),
    )
    .await
    .expect("a cancellation must not wait on the pool it is unblocking");
}

/// The three regions must not overlap, or a cancellation and a
/// subscription can be handed the same number.
#[tokio::test]
async fn the_regions_are_disjoint() {
    let pool = TagPool::new(4);
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..4 {
        assert!(seen.insert(pool.acquire_tag().await));
    }
    for _ in 0..4 {
        let tag = pool.acquire_streaming_tag().await;
        assert!(seen.insert(tag), "streaming tag {tag} collided");
        assert!(pool.is_streaming_tag(tag));
    }
    for _ in 0..4 {
        let tag = pool.acquire_control_tag().await;
        assert!(seen.insert(tag), "control tag {tag} collided");
        assert!(!pool.is_streaming_tag(tag));
    }
}

/// A released tag must go back to the region it came from, or the
/// regions leak into each other over time.
#[tokio::test]
async fn a_tag_returns_to_its_own_region() {
    let pool = TagPool::new(2);
    let streaming = pool.acquire_streaming_tag().await;
    pool.release_tag(streaming).await;
    assert!(pool.is_streaming_tag(pool.acquire_streaming_tag().await));
    assert_eq!(pool.acquire_tag().await, 1, "the ordinary region is intact");
}

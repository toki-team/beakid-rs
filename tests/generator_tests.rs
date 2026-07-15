use beakid::{BeakId, BeakIdGenerator};
use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const EPOCH_MS: i64 = 1_750_000_000_000;

fn test_epoch() -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(EPOCH_MS as u64)
}

fn make_generator(worker_id: u16) -> BeakIdGenerator {
    BeakIdGenerator::new(worker_id, test_epoch(), Duration::from_secs(1))
}

fn relative_now() -> i64 {
    SystemTime::now()
        .duration_since(test_epoch())
        .unwrap()
        .as_millis() as i64
}

// ========== BeakId field extraction ==========

#[tokio::test]
async fn raw_is_non_negative() {
    for worker_id in [0, 1, 512, 1023] {
        let gen = make_generator(worker_id);
        let id = gen.next_id().await;
        assert!(
            id.raw() >= 0,
            "raw() must be non-negative, got {}",
            id.raw()
        );
    }
}

#[tokio::test]
async fn worker_id_matches_constructor() {
    for worker_id in [0, 1, 42, 512, 1023] {
        let gen = make_generator(worker_id);
        let id = gen.next_id().await;
        assert_eq!(
            id.worker_id(),
            worker_id,
            "worker_id mismatch for input {}",
            worker_id
        );
    }
}

#[tokio::test]
async fn worker_id_zero() {
    let gen = make_generator(0);
    let id = gen.next_id().await;
    assert_eq!(id.worker_id(), 0);
}

#[tokio::test]
async fn worker_id_max_1023() {
    let gen = make_generator(1023);
    let id = gen.next_id().await;
    assert_eq!(id.worker_id(), 1023);
}

#[tokio::test]
async fn timestamp_roughly_matches_now() {
    let gen = make_generator(0);
    let before = relative_now();
    let id = gen.next_id().await;
    let after = relative_now();
    let ts = id.timestamp();
    assert!(
        ts >= before - 1,
        "timestamp {} should be >= before {}",
        ts,
        before
    );
    assert!(
        ts <= after + 1,
        "timestamp {} should be <= after {}",
        ts,
        after
    );
}

#[tokio::test]
async fn reserved_bit_is_zero() {
    for worker_id in [0, 1, 1023] {
        let gen = make_generator(worker_id);
        let id = gen.next_id().await;
        let raw = id.raw();
        assert!(raw >= 0, "raw must be non-negative, bit 63 must be 0");
        assert_eq!(raw >> 63, 0, "reserved bit 63 must be 0, got: {:#b}", raw);
    }
}

// ========== BeakId traits ==========

#[tokio::test]
async fn copy_and_clone() {
    let gen = make_generator(0);
    let id = gen.next_id().await;
    let copied: BeakId = id;
    assert_eq!(id, copied);
    let cloned: BeakId = id.clone();
    assert_eq!(id, cloned);
}

#[tokio::test]
async fn eq_and_ne() {
    let gen = make_generator(0);
    let id1 = gen.next_id().await;
    let id2 = gen.next_id().await;
    assert_eq!(id1, id1);
    assert_ne!(id1, id2);
}

#[tokio::test]
async fn ord_comparisons() {
    let gen = make_generator(0);
    let id1 = gen.next_id().await;
    let id2 = gen.next_id().await;

    assert!(id1 < id2);
    assert!(id2 > id1);
    assert!(id1 <= id1);
    assert!(id1 <= id2);
    assert!(id2 >= id1);
    assert!(id1 >= id1);
}

#[tokio::test]
async fn partial_ord_some() {
    let gen = make_generator(0);
    let id1 = gen.next_id().await;
    let id2 = gen.next_id().await;
    assert_eq!(id1.partial_cmp(&id2), Some(std::cmp::Ordering::Less));
    assert_eq!(id2.partial_cmp(&id1), Some(std::cmp::Ordering::Greater));
    assert_eq!(id1.partial_cmp(&id1), Some(std::cmp::Ordering::Equal));
}

#[tokio::test]
async fn ord_max_min() {
    let gen = make_generator(0);
    let a = gen.next_id().await;
    let b = gen.next_id().await;
    let c = gen.next_id().await;

    assert_eq!(a.max(b), b);
    assert_eq!(a.min(b), a);
    assert_eq!(c.max(b), c);
    assert_eq!(c.min(b), b);
}

#[tokio::test]
async fn hash_consistent() {
    let gen = make_generator(0);
    let id1 = gen.next_id().await;
    let id1_copy: BeakId = id1;
    let id2 = gen.next_id().await;

    let hash1 = {
        let mut h = DefaultHasher::new();
        id1.hash(&mut h);
        h.finish()
    };
    let hash1_copy = {
        let mut h = DefaultHasher::new();
        id1_copy.hash(&mut h);
        h.finish()
    };
    let hash2 = {
        let mut h = DefaultHasher::new();
        id2.hash(&mut h);
        h.finish()
    };

    assert_eq!(hash1, hash1_copy, "same BeakId must hash equally");
    assert_ne!(hash1, hash2, "different BeakIds should hash differently");
}

#[tokio::test]
async fn from_beakid_for_i64() {
    let gen = make_generator(0);
    let id = gen.next_id().await;
    let raw = id.raw();
    let converted: i64 = id.into();
    assert_eq!(raw, converted);
}

#[tokio::test]
async fn debug_non_empty() {
    let gen = make_generator(0);
    let id = gen.next_id().await;
    let s = format!("{:?}", id);
    assert!(!s.is_empty());
}

#[tokio::test]
async fn display_non_empty() {
    let gen = make_generator(0);
    let id = gen.next_id().await;
    let s = format!("{}", id);
    assert!(!s.is_empty());
}

// ========== BeakId ordering logic ==========

#[tokio::test]
async fn ids_with_future_timestamp_are_greater() {
    let gen = make_generator(0);
    let base = relative_now();

    let id1 = gen.next_id_with_timestamp(base).await;
    let id2 = gen.next_id_with_timestamp(base + 1).await;
    let id3 = gen.next_id_with_timestamp(base + 10).await;

    assert!(id1.timestamp() <= id2.timestamp());
    assert!(id1.timestamp() <= id3.timestamp());
    assert!(id1 < id2);
    assert!(id2 < id3);
}

#[tokio::test]
async fn ids_with_same_timestamp_have_same_sequence() {
    let gen = make_generator(0);
    let base = relative_now();

    let a = gen.next_id_with_timestamp(base).await;
    let b = gen.next_id_with_timestamp(base).await;

    assert_eq!(a.timestamp(), base);
    assert_eq!(b.timestamp(), base);
    assert_eq!(a.sequence(), b.sequence());
}

// ========== BeakId bit layout ==========

#[tokio::test]
async fn bit_layout_matches_accessors() {
    let gen = make_generator(777);
    let id = gen.next_id().await;
    let raw = id.raw();

    let worker_from_bits = (raw & 0x3FF) as u16;
    let seq_from_bits = ((raw >> 10) & 0xFFF) as u16;
    let ts_from_bits = (raw >> 22) & 0x1FF_FFFF_FFFF;

    assert_eq!(worker_from_bits, id.worker_id(), "worker_id bit mismatch");
    assert_eq!(seq_from_bits, id.sequence(), "sequence bit mismatch");
    assert_eq!(
        ts_from_bits,
        id.timestamp() as i64,
        "timestamp bit mismatch"
    );
    assert_eq!(raw >> 63, 0, "reserved bit 63 must be 0");
}

// ========== Generator Send + Sync ==========

#[tokio::test]
async fn generator_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<BeakIdGenerator>();
}

// ========== Generator monotonicity & uniqueness ==========

#[tokio::test]
async fn monotonically_increasing() {
    let gen = make_generator(0);
    let mut prev = gen.next_id().await;
    for _ in 0..2000 {
        let next = gen.next_id().await;
        assert!(
            next.raw() > prev.raw(),
            "IDs must be strictly increasing: {} <= {}",
            next.raw(),
            prev.raw()
        );
        prev = next;
    }
}

#[tokio::test]
async fn unique_ids_sequential() {
    let gen = make_generator(0);
    let count = 2000;
    let mut seen = HashSet::with_capacity(count);
    for _ in 0..count {
        let id = gen.next_id().await;
        assert!(seen.insert(id.raw()), "duplicate ID: {}", id.raw());
    }
    assert_eq!(seen.len(), count);
}

#[tokio::test(flavor = "multi_thread")]
async fn unique_ids_concurrent() {
    use std::sync::Arc;

    let gen = Arc::new(make_generator(7));
    let tasks = 8;
    let ids_per_task = 500;

    let mut handles = Vec::with_capacity(tasks);
    for _ in 0..tasks {
        let gen = Arc::clone(&gen);
        handles.push(tokio::spawn(async move {
            let mut ids = Vec::with_capacity(ids_per_task);
            for _ in 0..ids_per_task {
                ids.push(gen.next_id().await);
            }
            ids
        }));
    }

    let mut all = Vec::with_capacity(tasks * ids_per_task);
    for h in handles {
        all.extend(h.await.unwrap());
    }

    let unique: HashSet<i64> = all.iter().map(|id| id.raw()).collect();
    assert_eq!(unique.len(), all.len(), "concurrent IDs must be unique");
}

// ========== Generator next_id_with_timestamp ==========

#[tokio::test]
async fn next_id_with_timestamp_sets_exact_timestamp() {
    let gen = make_generator(0);
    let now = relative_now();
    let id = gen.next_id_with_timestamp(now).await;
    assert_eq!(id.timestamp(), now);
}

#[tokio::test]
async fn next_id_with_timestamp_higher_now_gives_higher_id() {
    let gen = make_generator(0);
    let base = relative_now();

    let id0 = gen.next_id_with_timestamp(base).await;
    let id1 = gen.next_id_with_timestamp(base + 1).await;
    let id2 = gen.next_id_with_timestamp(base + 2).await;

    assert!(id0 < id1);
    assert!(id1 < id2);
}

#[tokio::test]
async fn next_id_with_timestamp_past_timestamp_maintains_monotonicity() {
    let gen = make_generator(0);
    let base = relative_now();

    let id1 = gen.next_id_with_timestamp(base + 100).await;
    let id2 = gen.next_id_with_timestamp(base).await;

    assert!(
        id2.raw() > id1.raw(),
        "ID must increase even if now goes backwards"
    );
}

#[tokio::test]
async fn next_id_with_timestamp_worker_id_preserved() {
    for worker_id in [0, 42, 1023] {
        let gen = make_generator(worker_id);
        let base = relative_now();
        let id = gen.next_id_with_timestamp(base).await;
        assert_eq!(id.worker_id(), worker_id);
    }
}

// ========== Generator window_size sleep ==========

#[tokio::test]
async fn sleeps_when_timestamp_far_ahead_of_window() {
    let window = Duration::from_millis(5);
    let gen = BeakIdGenerator::new(0, test_epoch(), window);
    let base = relative_now();

    let _ = gen.next_id_with_timestamp(base + 200).await;

    let result =
        tokio::time::timeout(Duration::from_millis(50), gen.next_id_with_timestamp(base)).await;
    assert!(
        result.is_err(),
        "expected timeout: generator should be sleeping"
    );
}

#[tokio::test]
async fn no_sleep_when_within_window() {
    let window = Duration::from_millis(500);
    let gen = BeakIdGenerator::new(0, test_epoch(), window);
    let base = relative_now();

    let _ = gen.next_id_with_timestamp(base + 50).await;

    let result =
        tokio::time::timeout(Duration::from_millis(100), gen.next_id_with_timestamp(base)).await;
    assert!(
        result.is_ok(),
        "should not sleep when difference is within window"
    );
}

#[tokio::test]
async fn sleeps_when_clock_goes_backwards_beyond_window() {
    let window = Duration::from_millis(5);
    let gen = BeakIdGenerator::new(0, test_epoch(), window);
    let base = relative_now();

    let _ = gen.next_id_with_timestamp(base + 100).await;
    let _ = gen.next_id_with_timestamp(base + 200).await;

    let result =
        tokio::time::timeout(Duration::from_millis(30), gen.next_id_with_timestamp(base)).await;
    assert!(
        result.is_err(),
        "should sleep when clock goes backwards beyond window"
    );
}

// ========== Generator multiple worker IDs ==========

#[tokio::test]
async fn different_worker_ids_produce_different_raw() {
    let gen0 = make_generator(0);
    let gen1 = make_generator(1);

    let id0 = gen0.next_id().await;
    let id1 = gen1.next_id().await;

    assert_eq!(id0.worker_id(), 0);
    assert_eq!(id1.worker_id(), 1);

    assert_ne!(id0.raw(), id1.raw());
}

// ========== BeakId used as HashMap key ==========

#[tokio::test]
async fn beakid_as_hashmap_key() {
    use std::collections::HashMap;

    let gen = make_generator(0);
    let id1 = gen.next_id().await;
    let id2 = gen.next_id().await;

    let mut map = HashMap::new();
    map.insert(id1, "first");
    map.insert(id2, "second");

    assert_eq!(map.get(&id1), Some(&"first"));
    assert_eq!(map.get(&id2), Some(&"second"));
    assert_eq!(map.len(), 2);
}

// ========== BeakId used as HashSet element ==========

#[tokio::test]
async fn beakid_in_hashset() {
    let gen = make_generator(0);
    let id1 = gen.next_id().await;
    let id2 = gen.next_id().await;
    let id3 = gen.next_id().await;

    let set: HashSet<BeakId> = [id1, id2, id3].into_iter().collect();
    assert!(set.contains(&id1));
    assert!(set.contains(&id2));
    assert!(set.contains(&id3));
    assert_eq!(set.len(), 3);
}

// ========== BeakId sorting ==========

#[tokio::test]
async fn beakid_sorting() {
    let gen = make_generator(0);
    let mut ids = Vec::new();
    for _ in 0..100 {
        ids.push(gen.next_id().await);
    }

    let mut sorted = ids.clone();
    sorted.sort();

    assert_eq!(ids, sorted, "IDs should already be in sorted order");
}

#[tokio::test]
async fn beakid_sort_unstable() {
    let gen = make_generator(0);
    let mut ids = Vec::new();
    for _ in 0..100 {
        ids.push(gen.next_id().await);
    }

    ids.reverse();
    ids.sort_unstable();

    for i in 1..ids.len() {
        assert!(ids[i - 1] < ids[i]);
    }
}

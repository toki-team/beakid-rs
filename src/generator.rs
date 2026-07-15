use std::{
    sync::atomic::{AtomicI64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::BeakId;

const TIMESTAMP_SHIFT: u8 = 22;
const SEQUENCE_ONE: i64 = 1 << 10;
const TIMESTAMP_MASK: i64 = (1 << 41) - 1;

pub struct BeakIdGenerator {
    id: AtomicI64,
    epoch_ms: i64,
    window_size_ms: i64,
    worker_id: i64,
}

impl BeakIdGenerator {
    pub fn new(worker_id: u16, epoch: SystemTime, window_size: Duration) -> Self {
        let epoch_ms = epoch
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_millis() as i64;

        let window_size_ms = window_size.as_millis() as i64;

        let initial_id =
            (now_ms(epoch_ms) << TIMESTAMP_SHIFT) | SEQUENCE_ONE | (worker_id as i64);

        Self {
            id: AtomicI64::new(initial_id),
            epoch_ms,
            window_size_ms,
            worker_id: worker_id as i64,
        }
    }

    pub async fn next_id(&self) -> BeakId {
        let now = now_ms(self.epoch_ms);
        self.next_id_with_timestamp(now).await
    }

    pub async fn next_id_with_timestamp(&self, now: i64) -> BeakId {
        let old = self.id.fetch_add(1, Ordering::Relaxed);
        let old_ts = (old >> TIMESTAMP_SHIFT) & TIMESTAMP_MASK;

        if old_ts >= now {
            let diff = old_ts - now;
            if diff >= self.window_size_ms {
                let sleep_ms = (diff - self.window_size_ms) as u64;
                tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
            }
            return BeakId::from_raw(old);
        }

        advance(&self.id, now, self.worker_id);
        BeakId::from_raw(old)
    }
}

fn advance(id: &AtomicI64, now_ms: i64, worker_id: i64) {
    let new_base = (now_ms << TIMESTAMP_SHIFT) | SEQUENCE_ONE | worker_id;
    loop {
        let current = id.load(Ordering::Relaxed);
        let current_ts = (current >> TIMESTAMP_SHIFT) & TIMESTAMP_MASK;
        if current_ts >= now_ms {
            break;
        }
        match id.compare_exchange_weak(current, new_base, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(_) => continue,
        }
    }
}

fn now_ms(epoch_ms: i64) -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as i64
        - epoch_ms
}

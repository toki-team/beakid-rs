<p align="center">
  <b>🇬🇧 <a href="README.md">English</a></b>
  &nbsp;&nbsp;|&nbsp;&nbsp;
  <b>🇷🇺 <a href="README.ru.md">Русский</a></b>
</p>

---

# beakid

[![Crates.io](https://img.shields.io/crates/v/beakid)](https://crates.io/crates/beakid)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Lock-free Snowflake-like 64-bit unique ID generator. Produces `i64` values compatible with PostgreSQL `BIGINT` columns. Built on **tokio** — `next_id()` is an async function that may call `tokio::time::sleep` when the clock drifts.

```toml
[dependencies]
beakid = "0.3"
```

## 1. Tokio dependency

The generator **requires tokio**. The public API is fully `async`:

| What | Why |
|---|---|
| `next_id` / `next_id_with_timestamp` | both are `async fn` |
| Time-drift sleep | uses `tokio::time::sleep` |
| Runtime | caller provides the tokio runtime |

Add tokio to your project:

```toml
[dependencies]
tokio = { version = "1", features = ["time"] }
beakid = "0.3"
```

For tests you'll also need `rt` + `macros`:

```toml
[dev-dependencies]
tokio = { version = "1", features = ["rt", "rt-multi-thread", "macros", "time"] }
```

## 2. Usage

```rust
use std::time::{Duration, SystemTime};
use beakid::BeakIdGenerator;

#[tokio::main]
async fn main() {
    let generator = BeakIdGenerator::new(
        1,                          // worker_id (0..1023)
        SystemTime::UNIX_EPOCH,     // custom epoch
        Duration::from_secs(1),     // clock-drift window
    );

    let id = generator.next_id().await;
    println!("{id}");               // 13510798882111488
    println!("{:?}", id);           // BeakId(13510798882111488)
    println!("{}", id.raw());       // 13510798882111488
}
```

### Reusing a timestamp

If you already have a timestamp for another task, pass it to avoid an extra syscall:

```rust
let now_ms = /* timestamp obtained elsewhere */;
let id = generator.next_id_with_timestamp(now_ms).await;
```

The `unix_ms` parameter is a **regular Unix timestamp in milliseconds** (as returned by `SystemTime::now()`); the generator converts it to its own epoch internally.

### Multi-instance setup

Use a different `worker_id` per process / pod:

```rust
let gen_a = BeakIdGenerator::new(1, epoch, window);
let gen_b = BeakIdGenerator::new(2, epoch, window);
let gen_c = BeakIdGenerator::new(3, epoch, window);
```

IDs from different generators are unique thanks to distinct `worker_id` bits, but they are **not strictly ordered across generators** — only within a single generator.

### Reading fields from an ID

```rust
let id = generator.next_id().await;

id.timestamp();   // 41-bit timestamp (ms since epoch)
id.sequence();    // 12-bit monotonic counter
id.worker_id();   // 10-bit instance identifier
id.raw();         // raw i64
```

## 3. ID structure

A 64-bit signed integer (`i64`), always non-negative:

```
┌──────────┬──────────────────────────────┬──────────────┬──────────────┐
│ reserved │         timestamp            │   sequence   │  worker_id   │
│  1 bit   │          41 bits             │   12 bits    │   10 bits    │
└──────────┴──────────────────────────────┴──────────────┴──────────────┘
 bit 63    bit 62                        bit 22  bit 21 bit 10  bit 9 bit 0
```

| Field | Bits | Range | Description |
|---|---|---|---|
| `reserved` | 1 (bit 63) | always `0` | Keeps IDs in the positive `i64` range |
| `timestamp` | 41 (bits 22–62) | 0 … 2⁴¹−1 | Milliseconds since custom `epoch`. Covers ~69 years |
| `sequence` | 12 (bits 10–21) | 0 … 4095 | Monotonic counter per millisecond |
| `worker_id` | 10 (bits 0–9) | 0 … 1023 | Instance / pod identifier |

The `reserved` bit ensures all generated IDs satisfy `0 <= id < 2^63`, mapping cleanly to PostgreSQL `BIGINT`.

## 4. Generator internals

### Lock-free algorithm

The generator uses a single `AtomicI64` — no `Mutex`, no `RwLock`. The core operation is one `fetch_add(1, Relaxed)`:

```text
                 ┌───────────────────┐
                 │  fetch_add(1)     │  ← atomic, lock-free
                 │  → old value = ID │
                 └────────┬──────────┘
                          │
                          ▼
                 ┌───────────────────┐
                 │  extract timestamp│  ← non-atomic check
                 │  from old value   │
                 └────────┬──────────┘
                          │
            ┌─────────────┴─────────────┐
            ▼                           ▼
   ┌─────────────────┐         ┌─────────────────┐
   │ ts >= now ?      │         │ now > ts ?       │
   │ diff < window ?  │         │ (clock lagging)  │
   └────┬────────┬────┘         └────────┬────────┘
        │        │                       │
        │    OK │           sleep        │  advance
        │       ▼              │         ▼
        │    ┌─────────┐       │  ┌─────────────────────┐
        │    │ return   │       │  │ CAS loop:           │
        │    │ old      │       │  │ set base = (now << 22) │
        │    └─────────┘       │  │   | (1 << 10) | worker  │
        │                      │  └─────────────────────┘
        ▼                      ▼
   ┌─────────────────────────────────────┐
   │ diff >= window ⇒ sleep(diff - window) │
   │ then return (no re-check)            │
   └─────────────────────────────────────┘
```

### Three states

| State | Condition | Action |
|---|---|---|
| **In window** | `0 <= ts - now < window_size` | Return ID immediately |
| **Ahead** | `ts - now >= window_size` | Sleep until the real clock catches up, then return |
| **Behind** | `now > ts` | CAS-advance the atomic to the current timestamp. Another thread may have already done this — CAS loop handles contention |

### Key properties

- **Monotonic within a generator**: IDs always increase within one `BeakIdGenerator`.
- **Not ordered across generators**: two instances with different `worker_id` values may produce interleaved IDs.
- **Gaps are normal**: the atomic counter jumps forward on `advance`, skipping unused values. This is by design — uniqueness is guaranteed, density is not.
- **Thread-safe & lock-free**: multiple `tokio` tasks can call `next_id` concurrently on the same `&BeakIdGenerator`.
- **PostgreSQL-ready**: IDs are non-negative `i64` values that fit directly into `BIGINT` columns.

### Clock drift protection

When a generated ID carries a timestamp **ahead** of the real clock (e.g. after an NTP correction, or due to a burst that consumed future timestamps), the generator **sleeps** using `tokio::time::sleep` until the real time catches up. The `window_size` parameter controls how much drift is tolerated before sleeping. A typical value is 1–5 seconds.

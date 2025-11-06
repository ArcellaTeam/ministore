# `ministate`

**Minimal state manager with durable WAL logging**  
*Part of the [`mini-rs`](https://github.com/ArcellaTeam/mini-rs) embeddable toolkit*

[![crates.io](https://img.shields.io/crates/v/ministate.svg)](https://crates.io/crates/ministate)  
[![docs.rs](https://img.shields.io/docsrs/ministate)](https://docs.rs/ministate)  
[![License: Apache-2.0/MIT](https://img.shields.io/badge/license-Apache%202.0%20%7C%20MIT-blue)](https://github.com/ArcellaTeam/mini-rs)

> `ministate` gives you just enough: crash-safe, in-memory state with replayable history.

---

## 🎯 Purpose

`ministate` provides a simple yet robust way to maintain **mutable application state** that survives process restarts, using an **append-only Write-Ahead Log (WAL)** built on [`ministore`](https://crates.io/crates/ministore).

Key features:
- ✅ **In-memory state** — fast reads via `RwLock`, full `Clone` on demand.
- ✅ **Durable mutations** — every change is `fsync`ed before being applied.
- ✅ **Crash recovery** — full state restored by replaying WAL on startup.
- ✅ **Logical sequencing** — each mutation gets a monotonically increasing sequence number.
- ✅ **Optional snapshots** — enable `snapshot` feature for faster recovery (via [`minisnap`](https://crates.io/crates/minisnap)).

Perfect for:
- Component/deployment registries (e.g., in [Arcella](https://github.com/ArcellaTeam/arcella))
- Queue metadata (e.g., in [walmq](https://github.com/ArcellaTeam/walmq))
- Local coordination primitives (leader election, locks)
- Embedded/IoT apps requiring crash-safe state

---

## 📦 Quick Start

### Basic (WAL-only)

```toml
[dependencies]
ministate = "0.1"
serde = { version = "1.0", features = ["derive"] }
```

```rust
use ministate::{Mutator, StateManager};
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Serialize, Deserialize)]
struct Counter { value: u32 }

#[derive(Serialize, Deserialize)]
struct Inc { by: u32 }

impl Mutator<Counter> for Inc {
    fn apply(&self, state: &mut Counter) {
        state.value += self.by;
    }
}

let mgr = StateManager::open("./state", "counter.wal.jsonl").await?;
mgr.apply(Inc { by: 10 }).await?;
assert_eq!(mgr.snapshot().await.value, 10);
```

### With Snapshots (opt-in)

```toml
[dependencies]
ministate = { version = "0.1", features = ["snapshot"] }
minisnap = "0.1"
```

```rust
mgr.create_snapshot().await?; // saves state + sequence number
```

---

## ✅ Guarantees

| Guarantee       | Description |
|-----------------|-------------|
| **Durability**  | If `apply().await` returns `Ok`, the mutation is on disk. |
| **Atomicity**   | In-memory state is updated **only after** WAL write succeeds. |
| **Ordering**    | Mutations applied in exact WAL order. |
| **Recoverability** | Full state restored from WAL (or WAL + snapshot). |

---

## 🧩 Part of `mini-rs`

- **`ministore`** — durable WAL engine
- **`minisnap`** — optional snapshot & log compaction support
- **`ministate`** — state manager (this crate)
- **`miniqueue`** — durable message queue (in development)

Used in:
- [**Arcella**](https://github.com/ArcellaTeam/arcella) — component and deployment state
- [**walmq**](https://github.com/ArcellaTeam/walmq) — message queue metadata

---

## 📄 License

Dual-licensed under:
- **Apache License 2.0**
- **MIT License**

Choose the one that best fits your project.

---

> **`ministate`** — because state should be simple, durable, and recoverable.  
> Part of the [`mini-rs`](https://github.com/ArcellaTeam/mini-rs) family: simple, embeddable, reliable.

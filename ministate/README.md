# `ministate` — A Minimal State Manager with Write-Ahead Logging (WAL)

**Durable, recoverable state for embedded and edge applications.**

> “State is complexity. But if you must have it, let it be simple and reliable.”

[![crates.io](https://img.shields.io/crates/v/ministate.svg)](https://crates.io/crates/ministate)  
[![docs.rs](https://img.shields.io/docsrs/ministate)](https://docs.rs/ministate)  
[![License: Apache-2.0/MIT](https://img.shields.io/badge/license-Apache%202.0%20%7C%20MIT-blue)](https://github.com/ArcellaTeam/mini-rs)

---

## 📦 Installation

```toml
[dependencies]
ministate = "0.1"
```

Optionally, with snapshot support:

```toml
[dependencies]
ministate = { version = "0.1", features = ["snapshot"] }
minisnap = "0.1"  # required separately for snapshot-based recovery
```

---

## 🚀 Quick Start

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

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();

    // Restores state from WAL (or initializes fresh)
    let mgr = StateManager::open(dir, "counter.wal.jsonl").await?;

    // Mutation — durably persisted
    mgr.apply(Inc { by: 10 }).await?;

    // Reading — fast and concurrent
    assert_eq!(mgr.snapshot().await.value, 10);

    Ok(())
}
```

---

## ✨ Features

- **Durable by default**: every mutation → `fsync` → **then** in-memory update.
- **Full recovery**: restart → automatic WAL replay → exact prior state.
- **Human-readable journal**: `JSONL` format — easy to inspect (`cat journal.wal.jsonl | jq`).
- **Logical sequencing**: each mutation gets a unique, monotonically increasing sequence number (`sequence()`).
- **Concurrency-safe**: reads (`snapshot()`) are concurrent; writes (`apply()`) are serialized.
- **No hidden threads**: fully `async/await`—no background tasks or magic.

### Optional: Snapshots (`snapshot` feature)

- Save full state snapshots: `mgr.create_snapshot().await?`
- Speed up recovery (future versions will support WAL compaction based on snapshots).

---

## 🔒 Guarantees

| Guarantee | Description |
|---------|-------------|
| **Durability** | If `apply().await` returns `Ok`, the data is on disk. |
| **Atomicity** | In-memory state is updated **only after** successful WAL write. |
| **Ordering** | Mutations are applied in the exact order recorded in the WAL. |
| **Recoverability** | Full state can be reconstructed from the WAL (or WAL + snapshot). |

---

## 🧱 Use Cases

- Component and deployment registries (e.g., in **[Arcella](https://github.com/ArcellaTeam/arcella)**)
- Queue metadata and coordination state (e.g., in **[walmq](https://github.com/ArcellaTeam/walmq)**)
- Local coordination primitives (leader election, locks, etc.)
- Embedded and IoT applications requiring crash-safe state

---

## 📚 Documentation

Full API documentation is available on [docs.rs/ministate](https://docs.rs/ministate).

---

## 📄 License

Dual-licensed under **MIT** and **Apache-2.0** — choose the one that best fits your project.

---

> **`ministate`** — because state should be simple, durable, and recoverable.  
> Part of the [`mini-rs`](https://github.com/ArcellaTeam/mini-rs) family.
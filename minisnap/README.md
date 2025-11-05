# `ministore`

**Minimal snapshot store for durable state managers**  
*Part of the [`mini-rs`](https://github.com/ArcellaTeam/mini-rs) embeddable toolkit*

[![crates.io](https://img.shields.io/crates/v/minisnap.svg)](https://crates.io/crates/minisnap)  
[![docs.rs](https://img.shields.io/docsrs/minisnap)](https://docs.rs/minisnap)  
[![License: Apache-2.0/MIT](https://img.shields.io/badge/license-Apache%202.0%20%7C%20MIT-blue)](https://github.com/ArcellaTeam/mini-rs)

> `minisnap` gives you just enough: atomic, human-readable snapshots with zero hidden machinery.

---

## 🎯 Purpose

`minisnap` provides a simple, reliable way to **serialize and restore** a full copy of your application state to and from disk. It is designed to complement WAL-based systems like [`ministore`](https://crates.io/crates/ministore) and [`ministate`](https://crates.io/crates/ministate) by enabling:

- ✅ **Fast recovery** (skip replaying a long WAL)
- 🔒 **Consistency tracking** via logical sequence numbers
- 🧪 **Human-readable snapshots** (plain JSON + text metadata)
- 🔄 **Future WAL compaction** (truncate log after snapshot)

Snapshots are **explicit** — you decide when to create them. No background threads. No magic.

---

## 📦 Quick Start

### Add to `Cargo.toml`

```toml
[dependencies]
minisnap = "0.1"
serde = { version = "1.0", features = ["derive"] }
```

### Basic Usage

```rust
use minisnap::SnapStore;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct AppState {
    counter: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = SnapStore::new("./state");

    // Save a snapshot with sequence number (e.g., last WAL index)
    let state = AppState { counter: 42 };
    store.create(&state, 10).await?;

    // Restore later (e.g., on startup)
    let (restored, seq): (AppState, u64) = store.restore().await?;
    assert_eq!(restored, state);
    assert_eq!(seq, 10);

    Ok(())
}
```

> 💡 Snapshots are stored as two files:
> - `snapshot.json` — serialized state (pretty-printed JSON)
> - `snapshot.seq` — plain-text sequence number (e.g., `10`)

---

## ✨ Features

- **Atomic writes**: Temporary files + `rename()` ensure snapshots are never partial.
- **Type-safe**: Fully integrates with `serde`; errors are explicit.
- **Filesystem-safe**: Works on any POSIX-compliant filesystem (ext4, XFS, APFS, etc.).
- **Zero dependencies** beyond `serde` and `tokio` (only `fs` and `io-util` features).
- **< 200 lines of core logic** — easy to audit and understand.

---

## 🔗 Integration

`minisnap` is **optionally used** by [`ministate`](https://crates.io/crates/ministate) when the `snapshot` Cargo feature is enabled:

```toml
ministate = { version = "0.1", features = ["snapshot"] }
```

This allows `ministate` to:
- Save snapshots via `StateManager::create_snapshot()`
- (Future) Recover from snapshot + tail of WAL
- (Future) Compact WAL safely

---

## 🛡 Guarantees

| Property        | Guarantee |
|-----------------|-----------|
| **Durability**  | After `create().await`, snapshot is on disk (`fsync` is implicit in `tokio::fs::write` + atomic rename on most systems). |
| **Atomicity**   | Readers always see either the old snapshot or the new one — never a corrupt intermediate state. |
| **Consistency** | Each snapshot is paired with a user-provided sequence number (e.g., last applied WAL index). |

> ⚠️ **Note**: Atomic rename is **not guaranteed** on FAT32 or some network filesystems. Use on reliable local storage.

---

## 🧪 Testing & Reliability

- Full test coverage for happy and error paths
- Integration-tested with `ministate`
- Used in [**Arcella**](https://github.com/ArcellaTeam/arcella) (modular WebAssembly platform) and [**walmq**](https://github.com/ArcellaTeam/walmq) (WAL-based message queue)

---

## 📄 License

Dual-licensed under:
- **Apache License 2.0**
- **MIT License**

Choose the one that best fits your project.

---

> **`minisnap`** — because sometimes, the best state is the one you can **trust after a crash**.  
> Part of the [`mini-rs`](https://github.com/ArcellaTeam/mini-rs) family: simple, embeddable, reliable.

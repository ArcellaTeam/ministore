# `minisnap`

**Minimal snapshot store for durable state managers**  
*Part of the [`mini-rs`](https://github.com/ArcellaTeam/mini-rs) embeddable toolkit*

[![crates.io](https://img.shields.io/crates/v/minisnap.svg)](https://crates.io/crates/minisnap)  
[![docs.rs](https://img.shields.io/docsrs/minisnap)](https://docs.rs/minisnap)  
[![License: Apache-2.0/MIT](https://img.shields.io/badge/license-Apache%202.0%20%7C%20MIT-blue)](https://github.com/ArcellaTeam/mini-rs)

> `minisnap` gives you just enough: atomic, human-readable snapshots with zero hidden machinery.

---

## 🎯 Purpose

`minisnap` provides a simple, reliable way to **serialize and restore** a full copy of your application state to and from disk. It is designed to complement WAL-based systems like [`ministore`](https://crates.io/crates/ministore) by enabling:

- ✅ **Fast recovery** (skip replaying a long WAL)
- 🔒 **Consistency tracking** via logical sequence numbers
- 🧑‍💻 **Human-readable snapshots** (plain JSON + text metadata)
- 🔮 **Future WAL compaction** (truncate log after snapshot)

Snapshots are **explicit** — you decide when to create them. No background threads. No magic.

---

## 📦 Quick Start

```toml
[dependencies]
minisnap = "0.1"
serde = { version = "1.0", features = ["derive"] }
```

```rust
use minisnap::SnapStore;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct AppState { counter: u64 }

let store = SnapStore::new("./state");

// Save
store.create(&AppState { counter: 42 }, 10).await?; // seq = 10

// Restore
let (restored, seq): (AppState, u64) = store.restore().await?;
assert_eq!(restored.counter, 42);
assert_eq!(seq, 10);
```

> 💡 Snapshots are stored as two files:  
> - `snapshot.json` — pretty-printed JSON state  
> - `snapshot.seq` — plain-text sequence number (e.g., `10`)

---

## ✅ Guarantees

| Property        | Guarantee |
|-----------------|-----------|
| **Durability**  | After `create().await`, snapshot is on disk (`fsync` via `tokio::fs::write` + atomic rename on POSIX). |
| **Atomicity**   | Temporary files + atomic `rename()` → no partial/corrupt snapshots. |
| **Consistency** | Each snapshot is paired with a user-provided sequence number (e.g., last WAL index). |

> ⚠️ Atomic `rename()` requires a POSIX-compliant filesystem (ext4, XFS, APFS). Avoid FAT32.

---

## 🧩 Part of `mini-rs`

- **`ministore`** — durable WAL engine
- **`minisnap`** — snapshotting & log compaction (this crate)
- **`ministate`** — uses `minisnap` when the `snapshot` Cargo feature is enabled
- **`miniqueue`** — future WAL compaction

Used in:
- [**Arcella**](https://github.com/ArcellaTeam/arcella) — component state snapshots
- [**walmq**](https://github.com/ArcellaTeam/walmq) — queue metadata snapshots

---

## 📄 License

Dual-licensed under:
- **Apache License 2.0**
- **MIT License**

Choose the one that best fits your project.

---

> **`minisnap`** — because sometimes, the best state is the one you can **trust after a crash**.  
> Part of the [`mini-rs`](https://github.com/ArcellaTeam/mini-rs) family: simple, embeddable, reliable.

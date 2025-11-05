# `ministore`

**A minimal WAL engine for durable, replayable event logging on a single node.**

[![crates.io](https://img.shields.io/crates/v/ministore.svg)](https://crates.io/crates/ministore)  
[![docs.rs](https://img.shields.io/docsrs/ministore)](https://docs.rs/ministore)  
[![License: Apache-2.0/MIT](https://img.shields.io/badge/license-Apache%202.0%20%7C%20MIT-blue)](https://github.com/ArcellaTeam/mini-rs)

`ministore` is **not a database** and **not a state manager**. It’s a **Write-Ahead Log (WAL) engine** that guarantees every record you write is:
- **Atomic** — written entirely or not at all.
- **Durable** — flushed to disk (`fsync`) before the function returns.
- **Replayable** — readable back in the exact order it was written.

Perfect for:
- Event sourcing and state machine replication
- Storing critical metadata (e.g., component registries in [Arcella](https://github.com/ArcellaTeam/arcella))
- Local message queues, caches, or logs on edge/IoT devices

The log is stored in **human-readable JSONL** format—easy to inspect and debug with standard tools like `cat`, `grep`, or `jq`.

---

## 📦 Installation

```toml
[dependencies]
ministore = "0.1"
serde = { version = "1.0", features = ["derive"] }
```

---

## 🚀 Quick Start

```rust
use ministore::MiniStore;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
enum CounterOp {
    Set { value: u32 },
    Inc { delta: u32 },
}

// Writing
let mut store = MiniStore::open("counter.log").await?;
store.append(&CounterOp::Set { value: 100 }).await?;
store.append(&CounterOp::Inc { delta: 25 }).await?;

// Replaying
let ops: Vec<CounterOp> = MiniStore::replay("counter.log").await?;
let mut value = 0;
for op in ops {
    match op {
        CounterOp::Set { value: v } => value = v,
        CounterOp::Inc { delta } => value += delta,
    }
}
assert_eq!(value, 125);
```

---

## ✅ Guarantees

- **Durability**: After `append().await`, data survives crashes and power loss.
- **Ordering**: Records replay in the exact order they were written.
- **Human-readable**: Log files are plain-text [JSONL](http://jsonlines.org/).
- **Embeddable**: < 300 lines of core logic, no background tasks or macros.

---

## 🧱 Part of `mini-rs`

`ministore` is the first library in the [`mini-rs`](https://github.com/ArcellaTeam/mini-rs) family:
- **`ministore`** — durable WAL engine (this crate)
- **`minisnap`** — snapshotting & log compaction (in development)
- **`ministate`** — ready-to-use state manager built on `ministore` + `minisnap`
- **`miniqueue`** — local durable message queue

---

## 📄 License

Dual-licensed under **Apache 2.0** or **MIT** — choose what works best for your project.

---

> **Reliability grows from simplicity.**  
> Use `ministore` when your data matters more than raw speed.
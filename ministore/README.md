# `ministore`

**Minimal WAL engine for durable, replayable event logging**  
*Part of the [`mini-rs`](https://github.com/ArcellaTeam/mini-rs) embeddable toolkit*

[![crates.io](https://img.shields.io/crates/v/ministore.svg)](https://crates.io/crates/ministore)  
[![docs.rs](https://img.shields.io/docsrs/ministore)](https://docs.rs/ministore)  
[![License: Apache-2.0/MIT](https://img.shields.io/badge/license-Apache%202.0%20%7C%20MIT-blue)](https://github.com/ArcellaTeam/mini-rs)

> `ministore` gives you just enough: atomic, durable, human-readable WAL with zero hidden machinery.

---

## 🎯 Purpose

`ministore` is a **Write-Ahead Log (WAL) engine** that guarantees every record you write is:
- ✅ **Atomic** — written entirely or not at all.
- ✅ **Durable** — `fsync`ed to disk before the call returns.
- ✅ **Replayable** — readable back in exact order.
- ✅ **Human-readable** — stored in [JSONL](http://jsonlines.org/) format.

It is **not a state manager** — just a reliable log. You define the record type and apply it to your state.

Perfect for:
- Event sourcing and state machines
- Metadata stores (e.g., in [Arcella](https://github.com/ArcellaTeam/arcella))
- Local queues, caches, or audit logs on edge/IoT devices

---

## 📦 Quick Start

```toml
[dependencies]
ministore = "0.1"
serde = { version = "1.0", features = ["derive"] }
```

```rust
use ministore::MiniStore;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Set { value: u32 }

// Write
let mut store = MiniStore::open("state/journal.jsonl").await?;
store.append(&Set { value: 42 }).await?; // fsync guaranteed

// Replay
let records: Vec<Set> = MiniStore::replay("state/journal.jsonl").await?;
```

> 💡 Journal format:  
> ```text
> // MINISTORE JOURNAL v0.1.0
> {"Set":{"value":42}}
> ```

---

## ✅ Guarantees

| Property      | Guarantee |
|---------------|-----------|
| **Durability**| After `append().await`, data survives crashes and power loss. |
| **Atomicity** | Each record is one valid JSON line — no partial writes. |
| **Ordering**  | Records replay in exact append order. |
| **Format**    | Human-readable JSONL + magic header for validation. |

---

## 🧩 Part of `mini-rs`

- **`ministore`** — durable WAL engine (this crate)
- **`minisnap`** — snapshotting & log compaction
- **`ministate`** — ready-to-use state manager (built on `ministore` + `minisnap`)
- **`miniqueue`** — local durable message queue

Used in:
- [**Arcella**](https://github.com/ArcellaTeam/arcella) — component registry
- [**walmq**](https://github.com/ArcellaTeam/walmq) — message broker metadata

---

## 📄 License

Dual-licensed under:
- **Apache License 2.0**
- **MIT License**

Choose the one that best fits your project.

---

> **`ministore`** — because if your data matters, it must survive a crash.  
> Part of the [`mini-rs`](https://github.com/ArcellaTeam/mini-rs) family: simple, embeddable, reliable.
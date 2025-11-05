# mini-rs — Minimalist Embedded Libraries for Rust

**Reliable, lightweight, and embeddable components for building single-node fault-tolerant systems.**

> «Complexity kills. It sucks the life out of developers, it makes products difficult to plan, build and test, it introduces security challenges, and it causes end-user and administrator frustration.»  
> — *Ray Ozzie*

---

## 🧩 The `mini` Family

| Library | Purpose | Status |
|--------|---------|--------|
| **[`ministore`](./ministore/)** | WAL engine for durable, append-only journaling in human-readable JSONL format. | ✅ Ready |
| **[`minisnap`](./minisnap/)** | Snapshot management and WAL compaction. | 🚧 In development |
| **[`ministate`](./ministate/)** | Ready-to-use state manager built on `ministore`, with **optional `minisnap` support**. | ✅ Ready |
| **[`miniqueue`](./miniqueue/)** | Simple local message queue with durability guarantees via `ministore`. | 🚧 In development |

All libraries are:
- **Embeddable** — single binary, minimal dependencies (`serde`, `tokio`).
- **Reliable** — every write is synced to disk (`fsync`).
- **Simple** — < 500 lines of core logic, no background tasks, GC, or macros.
- **Human-readable** — logs are easy to inspect and debug (`cat`, `jq`).

---

## 🎯 Philosophy

Every library in `mini-rs` follows three core principles:

1. **Solves one problem** — and does it exceptionally well.
2. **Doesn’t hide complexity** — you always know what’s happening.
3. **Guarantees durability** — your data survives crashes.

We don’t build frameworks. We build **bricks** you can use to assemble anything — from a simple daemon to a fault-tolerant message broker.

> 💡 **`minisnap` is not a required dependency, but an optional accelerator.**  
> Use it only when your WAL grows too long.

---

## 📦 Quick Start

### `ministore`: Durable Logging

```rust
use ministore::MiniStore;

#[derive(serde::Serialize, serde::Deserialize)]
struct Set { value: u32 }

let mut store = MiniStore::open("state/journal.jsonl").await?;
store.append(&Set { value: 42 }).await?; // fsync() guaranteed

let records: Vec<Set> = MiniStore::replay("state/journal.jsonl").await?;
```

### `ministate`: State Management (with Snapshots)

```rust
use ministate::{StateManager, Mutator};

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
struct Counter { value: u32 }

#[derive(serde::Serialize, serde::Deserialize)]
struct Inc { by: u32 }

impl Mutator<Counter> for Inc {
    fn apply(&self, state: &mut Counter) {
        state.value += self.by;
    }
}

// Snapshot support enabled via Cargo feature: `ministate = { ..., features = ["snapshot"] }`
let manager = StateManager::open("state").await?;
manager.apply(Inc { by: 10 }).await?;
assert_eq!(manager.snapshot().value, 10);

// Manual snapshot when needed
#[cfg(feature = "snapshot")]
manager.create_snapshot().await?;
```

### `minisnap` (coming soon): Direct Snapshot Control

```rust
use minisnap::SnapStore;

let snap_store = SnapStore::new("state");
snap_store.create(&my_state).await?;
let (state, seq) = snap_store.restore().await?;
```

---

## 🌐 Where It’s Used

- **[Arcella](https://github.com/ArcellaTeam/arcella)** — modular WebAssembly application platform:
  - `ministate` + `minisnap` — for storing component registries and deployments (with compaction).
- **[walmq](https://github.com/ArcellaTeam/walmq)** — Lightweight WAL-based message broker: metadata in `ministore` + `minisnap` , messages in binary WAL.
- Local databases, caches, and brokers.
- IoT, edge, and embedded systems.
- Any application where **data integrity is critical** and **infrastructure is minimal**.

---

## 📚 Documentation

- [`ministore` documentation on docs.rs](https://docs.rs/ministore)
- [`ministate` documentation on docs.rs](https://docs.rs/ministate) *(coming soon)*

---

## 🤝 Contributions

We welcome:
- Bug reports
- API suggestions
- Usage examples
- Tests and documentation

**Not sure where to start?** Just describe your use case — we’ll help you find a solution.

---

## 📄 License

Dual-licensed:
- **Apache License 2.0**
- **MIT License**

Choose whichever fits your project best.

---

> **mini-rs** — because reliability is born from simplicity.  
> Build systems you can trust.
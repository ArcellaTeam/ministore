# `ministore` — Architecture

**A WAL-based embedded store for append-only logs of serializable records**

---

### 🎯 Purpose

To provide a **minimal, reliable, and embeddable** mechanism for:

1.  **Durable recording** of serializable records (mutations, events, commands) into an append-only journal.
2.  **Replaying** the entire journal as a stream of strongly-typed records.

> **`ministore` does not manage state.** It is solely responsible for **durability and replay**. The logic for applying records to in-memory state is the responsibility of the calling application.

---

### 🧩 Core Principles

1.  **Simplicity**: The core is under 150 lines of code, with no background tasks, compaction, or caching.
2.  **Reliability**: Every record is followed by an `fsync()`, guaranteeing its persistence on disk in case of a crash.
3.  **Type Safety**: Records are serialized and deserialized using `serde`.
4.  **Human-Readability**: The journal is stored in the JSONL (JSON Lines) format and can be easily inspected by hand.
5.  **Zero Runtime Overhead**: No garbage collector, background threads, or complex algorithms.

---

### 🏗️ Architecture

```
┌───────────────────────────┐
│        Application        │
├────────────┬──────────────┤
│ State      │  Records     │
│ Management │  (mutations, │
│            │   events)    │
└────────────┴──────────────┘
               │
               ▼
┌───────────────────────────┐
│        MiniStore          │
├────────────┬──────────────┤
│   append() │   replay()   │
└────────────┴──────────────┘
               │
               ▼
┌───────────────────────────┐
│      Journal (WAL)        │
│   (JSONL-formatted file)  │
└───────────────────────────┘
```

-   **Journal**: An append-only file, immutable after a record is written.
-   **`append()`**: Appends a serialized record to the end of the journal and performs an `fsync()`.
-   **`replay()`**: Reads the journal from disk and returns a `Vec` of deserialized records.

---

### 📜 Journal Format

The journal is a text file in **[JSONL](http://jsonlines.org/)** format with the following structure:

```text
// MINISTORE JOURNAL v0.1.0
{"EventType":"UserCreated","user_id":123}
{"EventType":"OrderPlaced","order_id":456}
```

1.  **Line 1**: A magic header that identifies the file as a `ministore` journal and specifies its version.
2.  **Line N (N ≥ 2)**: A single JSON-serialized record from a type `R: Serialize`.

This format is easy to debug and inspect using standard Unix tools (`cat`, `tail`, `jq`).

---

### 🔁 Guarantees

| Guarantee | Description |
| --------- | ----------- |
| **Atomicity** | Each `append()` call writes exactly one record (one JSON line). |
| **Durability** | After `append()` returns `Ok(())`, the record is guaranteed to be on stable storage. |
| **Ordering** | Records returned by `replay()` are in the exact same order they were appended. |
| **Replay Safety** | The magic header is validated on open, preventing the accidental reading of corrupted or unrelated files. |

---

### 📦 API

```rust
impl MiniStore {
    /// Opens (or creates) a journal at the specified path.
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self>;

    /// Appends a record to the journal and guarantees its durability (via fsync).
    pub async fn append<R>(&mut self, record: &R) -> Result<()>
    where
        R: Serialize;

    /// Replays all records from the journal at the specified path.
    pub async fn replay<R, P: AsRef<Path>>(path: P) -> Result<Vec<R>>
    where
        R: DeserializeOwned;
}
```

---

### 🚫 What `ministore` is NOT

-   **It is not a state manager.** You are responsible for deciding how to apply the replayed records to your application's state.
-   **It is not thread-safe out of the box.** For shared access, wrap it in a synchronization primitive like `Arc<Mutex<MiniStore>>`.
-   **It is not high-performance for writes.** Every `append()` performs an `fsync` — it is slow but safe.
-   **It does not support deletion or log compaction.** This is an application-level concern (e.g., via snapshotting).

---

### 💡 Typical Use Cases

-   **Event Sourcing**: Storing a durable history of domain model changes.
-   **Management Logs**: WAL for metadata, configuration, or registries.
-   **Audit Trails**: Immutable logs of operations for security and debugging.

> `ministore` is a **building block**, not a full database. It is ideal for scenarios where **simplicity and reliability** are paramount, not raw performance or complex queries.
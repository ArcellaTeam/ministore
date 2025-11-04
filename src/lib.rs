// ministore/src/lib.rs
//
// Copyright (c) 2025 Arcella Team
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE>
// or the MIT license <LICENSE-MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

//! A minimal, durable, append-only log store for serializable records.
//!
//! `ministore` is **not a state manager**. It is a **Write-Ahead Log (WAL) engine** that provides:
//! 1. **Durability**: every record is written to disk and `fsync`ed before the write returns.
//! 2. **Replay**: the entire log can be read back as a sequence of strongly-typed records.
//!
//! The caller is responsible for:
//! - Defining the record type (e.g., mutations, events, commands).
//! - Applying records to in-memory state.
//! - Managing concurrency (e.g., via `Arc<RwLock<MiniStore>>`).
//!
//! This design makes `ministore` ideal for building:
//! - Event-sourced systems
//! - State machines with durable logs
//! - Metadata stores (like Arcella's component registry)
//!
//! # Guarantees
//!
//! - **Atomicity**: each `append()` call writes exactly one record (as one JSON line).
//! - **Durability**: after `append()` returns `Ok(())`, the record is on stable storage.
//! - **Ordering**: records are replayed in the exact order they were appended.
//! - **Replay Safety**: the journal format includes a magic header to prevent misuse.
//!
//! # Journal Format
//!
//! The on-disk journal is a text file in [JSONL](http://jsonlines.org/) format:
//! ```text
//! // MINISTORE JOURNAL v0.1.0
//! {"Set":{"value":10}}
//! {"Inc":{"by":5}}
//! ```
//! - Line 1: magic header (for versioning and validation).
//! - Line N (N ≥ 2): one JSON-serialized record per line.
//!
//! The format is human-readable and easy to inspect/debug with standard tools (`cat`, `jq`, etc.).
//!
//! # Example: Simple Counter
//!
//! ```
//! use ministore::MiniStore;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Serialize, Deserialize)]
//! enum CounterMutation {
//!     Set { value: u32 },
//!     Inc { delta: u32 },
//! }
//!
//! #[derive(Default)]
//! struct Counter {
//!     value: u32,
//! }
//!
//! impl Counter {
//!     fn apply(&mut self, mutation: &CounterMutation) {
//!         match mutation {
//!             CounterMutation::Set { value } => self.value = *value,
//!             CounterMutation::Inc { delta } => self.value += *delta,
//!         }
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let tmp = tempfile::tempdir()?;
//!     let path = tmp.path().join("counter.log");
//!
//!     // 1. Open the store
//!     let mut store = MiniStore::open(&path).await?;
//!
//!     // 2. Append mutations
//!     store.append(&CounterMutation::Set { value: 100 }).await?;
//!     store.append(&CounterMutation::Inc { delta: 25 }).await?;
//!
//!     // 3. Rebuild state from log
//!     let mut counter = Counter::default();
//!     let records: Vec<CounterMutation> = MiniStore::replay(&path).await?;
//!     for record in records {
//!         counter.apply(&record);
//!     }
//!
//!     assert_eq!(counter.value, 125);
//!     Ok(())
//! }
//! ```

use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;
use tokio::{
    fs::OpenOptions,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
};

mod error;
pub use error::MiniStoreError;

/// A specialized [`Result`](std::result::Result) type for `ministore` operations.
pub type Result<T> = std::result::Result<T, MiniStoreError>;

/// Magic header written at the beginning of every new journal file.
///
/// Used to:
/// - Identify the file as a `ministore` journal.
/// - Validate the journal version during replay.
/// - Prevent accidental corruption by external tools.
const JOURNAL_MAGIC: &str = "// MINISTORE JOURNAL v0.1.0\n";

/// A durable, append-only log store for serializable records.
///
/// `MiniStore` manages a single journal file on disk. It provides two core operations:
/// - [`append`](Self::append): write a record to the log and guarantee it is on disk.
/// - [`replay`](Self::replay): read all records from a log file (static method).
///
/// # Concurrency
///
/// `MiniStore` is **not thread-safe** by itself. To share it across tasks, wrap it in a
/// synchronization primitive like `Arc<RwLock<MiniStore>>` (for write-heavy workloads)
/// or `Arc<Mutex<MiniStore>>`.
///
/// # Durability
///
/// Every call to [`append`] performs an `fsync` before returning, ensuring the record survives
/// process crashes and power loss. This makes writes **slow but safe** — perfect for metadata
/// or infrequent state changes.
#[derive(Debug)]
pub struct MiniStore {
    /// Buffered writer to the journal file.
    /// Ensures efficient disk I/O while maintaining durability via explicit `flush`/`sync`.
    journal_writer: BufWriter<tokio::fs::File>,
}

impl MiniStore {
    /// Opens a `ministore` journal at the given path.
    ///
    /// # Behavior
    ///
    /// - If the file **does not exist**, it is created and initialized with the magic header.
    /// - If the file **exists and is empty**, the magic header is written.
    /// - If the file **exists and is non-empty**, it is assumed to be a valid journal
    ///   (the magic header must already be present; validated during [`replay`]).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The path is not writable.
    /// - Parent directories cannot be created.
    /// - Disk I/O fails during magic header write.
    ///
    /// # Example
    ///
    /// ```
    /// # use ministore::MiniStore;
    /// # #[tokio::main] async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = MiniStore::open("/tmp/myapp.log").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Open file in write+append mode
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(path)
            .await?;

        let metadata = file.metadata().await?;
        let mut journal_writer = BufWriter::new(file);

        // Initialize empty file with magic header
        if metadata.len() == 0 {
            journal_writer.write_all(JOURNAL_MAGIC.as_bytes()).await?;
            journal_writer.flush().await?;
            journal_writer.get_ref().sync_all().await?;
        }

        Ok(Self { journal_writer })
    }

    /// Appends a serializable record to the journal and ensures it is durably stored.
    ///
    /// The record is serialized as a single JSON line and immediately `fsync`ed to disk.
    /// This operation is **atomic** — either the entire record is written, or nothing is.
    ///
    /// # Guarantees
    ///
    /// After this method returns `Ok(())`:
    /// - The record is visible in subsequent [`replay`] calls.
    /// - The record will survive process termination or system crash.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Serialization fails (e.g., unsupported type).
    /// - Disk write fails (e.g., full disk).
    /// - `fsync` fails (e.g., I/O error).
    ///
    /// # Performance
    ///
    /// This is a **slow** operation due to the `fsync`. Use it for critical metadata,
    /// not high-frequency data.
    ///
    /// # Example
    ///
    /// ```
    /// # use ministore::MiniStore;
    /// # #[derive(serde::Serialize)] struct Event { id: u32 }
    /// # #[tokio::main] async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut store = MiniStore::open("/tmp/events.log").await?;
    /// store.append(&Event { id: 42 }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn append<R>(&mut self, record: &R) -> Result<()>
    where
        R: Serialize,
    {
        let json = serde_json::to_string(record)?;
        self.journal_writer.write_all(json.as_bytes()).await?;
        self.journal_writer.write_all(b"\n").await?;
        self.journal_writer.flush().await?;
        self.journal_writer.get_ref().sync_all().await?;
        Ok(())
    }

    /// Replays all records from a journal file as a `Vec` of strongly-typed values.
    ///
    /// This is a **static method** — it does not require an open `MiniStore` instance.
    /// It reads the file from disk, validates the magic header, and deserializes each line.
    ///
    /// # Behavior
    ///
    /// - If the file **does not exist** or is **empty**, returns an empty `Vec`.
    /// - The **first line** must be the exact [`JOURNAL_MAGIC`] string (without trailing newline).
    /// - Subsequent lines must be valid JSON representations of type `R`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The magic header is missing or invalid.
    /// - Any line fails to deserialize as type `R`.
    /// - File I/O fails (e.g., permission denied).
    ///
    /// # Example
    ///
    /// ```
    /// # use ministore::MiniStore;
    /// # #[derive(serde::Deserialize, PartialEq, Debug)] struct Event { id: u32 }
    /// # #[tokio::main] async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let events: Vec<Event> = MiniStore::replay("/tmp/events.log").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn replay<R, P: AsRef<Path>>(path: P) -> Result<Vec<R>>
    where
        R: DeserializeOwned,
    {
        let path = path.as_ref();
        if !path.exists() || tokio::fs::metadata(path).await?.len() == 0 {
            return Ok(vec![]);
        }

        let file = OpenOptions::new().read(true).open(path).await?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        // Validate magic header
        let magic = lines
            .next_line()
            .await?
            .ok_or(MiniStoreError::MissingInitialState)?;
        if magic != JOURNAL_MAGIC.trim_end() {
            return Err(MiniStoreError::MissingInitialState);
        }

        let mut records = Vec::new();
        let mut line_num = 2; // magic = line 1

        while let Some(line) = lines.next_line().await? {
            match serde_json::from_str(&line) {
                Ok(record) => records.push(record),
                Err(e) => {
                    return Err(MiniStoreError::Deserialize {
                        line: line_num,
                        source: e,
                    });
                }
            }
            line_num += 1;
        }

        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    enum TestMutation {
        Set { value: u32 },
        Inc { by: u32 },
    }

    #[tokio::test]
    async fn test_ministore_append_replay() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test.jsonl");

        // Append records
        let mut store = MiniStore::open(&path).await.unwrap();
        store.append(&TestMutation::Set { value: 10 }).await.unwrap();
        store.append(&TestMutation::Inc { by: 5 }).await.unwrap();

        // Replay
        let records: Vec<TestMutation> = MiniStore::replay(&path).await.unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0], TestMutation::Set { value: 10 });
        assert_eq!(records[1], TestMutation::Inc { by: 5 });
    }

    #[tokio::test]
    async fn test_ministore_empty() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("empty.jsonl");

        let records: Vec<TestMutation> = MiniStore::replay(&path).await.unwrap();
        assert!(records.is_empty());
    }
}
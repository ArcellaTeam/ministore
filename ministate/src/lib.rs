// mini-rs/ministate/src/lib.rs
//
// Copyright (c) 2025 Arcella Team
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE>
// or the MIT license <LICENSE-MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

//! A minimal, in-memory state manager with durable WAL logging.
//!
//! `ministate` builds on top of [`ministore`] to provide:
//! - **In-memory state** (`S: Default + Clone + Serialize + Deserialize`)
//! - **Durable mutation logging** via append-only WAL
//! - **Replay on startup** to restore state
//! - **Logical sequence numbers** for ordering
//!
//! Optional integration with [`minisnap`] (via `snapshot` feature) enables:
//! - Fast state recovery from snapshots
//! - WAL compaction (truncate prefix after snapshot)
//!
//! # Design Philosophy
//!
//! - **No hidden background tasks** — everything is explicit.
//! - **No runtime overhead** when `snapshot` feature is disabled.
//! - **Human-readable WAL** — easy to inspect with `cat`, `jq`.
//! - **Single-writer, multi-reader** concurrency via `RwLock`.
//!
//! # Guarantees
//!
//! - **Durability**: after `apply().await` returns `Ok`, the mutation is guaranteed to be on disk.
//! - **Atomicity**: the in-memory state is updated **only if** the WAL write succeeds.
//! - **Ordering**: mutations are applied in the exact order they are logged.
//! - **Recoverability**: on restart, the state is fully reconstructed by replaying the WAL.
//!
//! # Example
//!
//! ```rust
//! use ministate::{Mutator, StateManager};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Default, Clone, Serialize, Deserialize)]
//! struct Counter { value: u32 }
//!
//! #[derive(Serialize, Deserialize)]
//! struct Inc { by: u32 }
//!
//! impl Mutator<Counter> for Inc {
//!     fn apply(&self, state: &mut Counter) {
//!         state.value += self.by;
//!     }
//! }
//!
//! #[tokio::main(flavor = "current_thread")]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let tmp = tempfile::tempdir()?;
//!     let dir = tmp.path();
//!
//!     let mgr = StateManager::open(dir, "counter.wal").await?;
//!     mgr.apply(Inc { by: 10 }).await?;
//!     assert_eq!(mgr.snapshot().await.value, 10);
//!     Ok(())
//! }
//! ```

use ministore::MiniStore;
use serde::{de::DeserializeOwned, Serialize};
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;

mod error;
pub use error::MiniStateError;

/// A mutation that can be applied to a state `S`.
///
/// User-defined types implement this trait to describe how they modify state.
///
/// # Safety
///
/// Implementations **must be pure and deterministic**:
/// - The same mutation applied to the same state **must always produce the same result**.
/// - No side effects (e.g., I/O, random number generation) are allowed.
pub trait Mutator<S> {
    /// Apply this mutation to the mutable state.
    ///
    /// This method is called **after** the mutation is durably logged to the WAL,
    /// so it is safe to assume the mutation will survive a crash.
    fn apply(&self, state: &mut S);
}

/// Manages in-memory state with durable WAL logging.
///
/// On creation, replays the journal to reconstruct state.
/// On `apply()`, logs the mutation and updates state atomically.
///
/// The manager is **safe to share across threads** (`Sync + Send`)
/// and supports **concurrent reads** (via `snapshot()`)
/// but **serializes writes** (via `RwLock` in `apply()`).
#[derive(Debug)]
pub struct StateManager<S, M> {
    /// The current in-memory state, protected by a reader-writer lock.
    state: RwLock<S>,
    /// The underlying durable WAL store.
    store: MiniStore,
    /// Logical sequence number: total number of successfully applied mutations.
    /// Starts at 0 for an empty state, increments by 1 after each successful `apply()`.
    seq: std::sync::atomic::AtomicU64,
    /// Base directory for state (used for future snapshots).
    state_dir: PathBuf,
    /// Full path to the journal file.
    journal_path: PathBuf,
}

impl<S, M> StateManager<S, M>
where
    S: Default + Clone + Serialize + DeserializeOwned,
    M: Mutator<S> + Serialize + DeserializeOwned,
{
    /// Opens a state manager from a state directory and a custom journal filename.
    ///
    /// The journal file will be located at `{state_dir}/{journal_file}`.
    ///
    /// If the journal does not exist, it is created with a magic header.
    /// If it exists, all records are replayed to reconstruct the current state.
    ///
    /// # Arguments
    ///
    /// - `state_dir`: Directory to store journal (and, in the future, snapshots).
    /// - `journal_file`: Name of the journal file (e.g., `"deployments.wal"`).
    ///
    /// # Returns
    ///
    /// A new `StateManager` with state reconstructed from the journal.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The state directory cannot be created.
    /// - The journal file is corrupt or contains invalid records.
    /// - Disk I/O fails.
    pub async fn open<P1, P2>(state_dir: P1, journal_file: P2) -> Result<Self, MiniStateError>
    where
        P1: AsRef<Path>,
        P2: AsRef<Path>,
    {
        let state_dir = state_dir.as_ref();
        let journal_file = journal_file.as_ref();

        tokio::fs::create_dir_all(state_dir).await?;

        let journal_path = state_dir.join(journal_file);
        let records: Vec<M> = MiniStore::replay(&journal_path).await?;

        let mut state = S::default();
        for record in &records {
            record.apply(&mut state);
        }

        let store = MiniStore::open(&journal_path).await?;
        let seq = std::sync::atomic::AtomicU64::new(records.len() as u64);

        Ok(Self {
            state: RwLock::new(state),
            store,
            seq,
            state_dir: state_dir.to_path_buf(),
            journal_path,
        })
    }

    /// Applies a mutation to the state **only after** it has been durably logged.
    ///
    /// Order of operations:
    /// 1. Serialize and append the mutation to the WAL.
    /// 2. `fsync` to ensure durability.
    /// 3. **Only on success** — apply the mutation to in-memory state.
    ///
    /// If WAL write fails, the mutation is **not applied** and an error is returned.
    ///
    /// This method **holds a write lock** on the state for its duration,
    /// so concurrent calls will be serialized.
    ///
    /// # Returns
    ///
    /// The new sequence number (1-based index of this mutation in the WAL).
    ///
    /// # Errors
    ///
    /// Returns an error if the mutation cannot be serialized or written to disk.
    pub async fn apply(&self, mutation: M) -> Result<u64, MiniStateError> {
        let mut state = self.state.write().await;

        // 1. Durable write to WAL
        self.store.append(&mutation).await?;

        // 2. Only now apply to in-memory state
        mutation.apply(&mut state);

        // 3. Increment sequence number
        let new_seq = self.seq.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        Ok(new_seq)
    }

    /// Returns a **clone** of the current in-memory state.
    ///
    /// This is a **read-only snapshot** — modifications to the returned value
    /// do not affect the managed state.
    ///
    /// This method **acquires a read lock**, so it is non-blocking for other readers
    /// and only blocks during concurrent `apply()` calls.
    pub async fn snapshot(&self) -> S {
        self.state.read().await.clone()
    }

    /// Returns the current sequence number (number of successfully applied mutations).
    ///
    /// Starts at `0` for empty state, `1` after first `apply()`, etc.
    ///
    /// This value is **monotonically increasing** and reflects the total number
    /// of mutations that have been durably logged and applied.
    pub fn sequence(&self) -> u64 {
        self.seq.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Returns the full path to the journal file.
    ///
    /// Useful for backup, inspection, or external tooling.
    pub fn journal_path(&self) -> &Path {
        &self.journal_path
    }
}

// === Future: minisnap integration (opt-in via feature) ===

#[cfg(feature = "snapshot")]
mod snapshot_support {
    use super::*;
    use minisnap::SnapStore;

    impl<S, M> StateManager<S, M>
    where
        S: Default + Clone + Serialize + DeserializeOwned,
        M: Mutator<S> + Serialize + DeserializeOwned,
    {
        /// Creates a snapshot of the current state and stores it in the state directory.
        ///
        /// The snapshot file is named `snapshot.json` (or configurable in future versions).
        ///
        /// **Note**: This does **not** truncate the WAL. WAL compaction must be done separately.
        ///
        /// Requires the `snapshot` feature.
        ///
        /// # Returns
        ///
        /// `Ok(())` if the snapshot was successfully written to disk.
        ///
        /// # Errors
        ///
        /// Returns an error if the state cannot be serialized or the snapshot file cannot be written.
        pub async fn create_snapshot(&self) -> Result<(), MiniStateError> {
            let snap_store = SnapStore::new(&self.state_dir);
            let seq = self.sequence();
            snap_store.create(&self.snapshot().await, seq).await?;
            Ok(())
        }

        // TODO: open_with_snapshot() — load snapshot + replay tail of WAL
        // TODO: compact() — truncate WAL up to last snapshot
    }
}

#[cfg(feature = "snapshot")]
pub use snapshot_support::*;
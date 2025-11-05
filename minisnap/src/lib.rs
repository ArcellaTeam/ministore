// mini-rs/minisnap/src/lib.rs
//
// Copyright (c) 2025 Arcella Team
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE>
// or the MIT license <LICENSE-MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

//! Minimal snapshot store for durable state managers.
//!
//! `minisnap` provides a simple, reliable way to **serialize and restore** a full copy
//! of application state to/from disk. It is designed to complement WAL-based systems
//! like [`ministore`] and [`ministate`] by enabling **fast recovery** and future **WAL compaction**.
//!
//! ## Features
//!
//! - **Explicit snapshotting**: You control when snapshots are created.
//! - **Atomic writes**: Snapshots are written to a temp file and atomically renamed.
//! - **Sequence tracking**: Each snapshot is associated with a logical sequence number
//!   (e.g., the last applied WAL index) for consistency.
//! - **Human-readable**: Snapshots are stored as plain JSON (via `serde`).
//!
//! ## Integration
//!
//! Intended for use with `ministate` (via the `snapshot` feature), but can be used standalone.
//!
//! # Example
//!
//! ```rust
//! use minisnap::SnapStore;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize, PartialEq, Debug)]
//! struct AppState { counter: u64 }
//!
//! #[tokio::main(flavor = "current_thread")]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let tmp = tempfile::tempdir()?;
//!     let store = SnapStore::new(tmp.path());
//!
//!     let state = AppState { counter: 42 };
//!     store.create(&state, 10).await?; // seq = 10
//!
//!     let (restored, seq) = store.restore().await?;
//!     let restored: AppState = restored;
//!     assert_eq!(restored, state);
//!     assert_eq!(seq, 10);
//!     Ok(())
//! }
//! ```

use serde::{de::DeserializeOwned, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

mod error;
pub use error::MiniSnapError;

/// A specialized [`Result`](std::result::Result) type for `minisnap` operations.
pub type Result<T> = std::result::Result<T, MiniSnapError>;

/// File name for the snapshot data.
const SNAPSHOT_FILE: &str = "snapshot.json";

/// File name for the snapshot metadata (e.g., sequence number).
const METADATA_FILE: &str = "snapshot.seq";

/// Manages snapshot storage in a dedicated directory.
///
/// The snapshot consists of two files:
/// - `{dir}/snapshot.json` — serialized state (JSON)
/// - `{dir}/snapshot.seq` — plain text sequence number (e.g., "12345")
///
/// Both files are written atomically and read consistently.
#[derive(Debug, Clone)]
pub struct SnapStore {
    dir: PathBuf,
    snapshot_path: PathBuf,
    metadata_path: PathBuf,
}

impl SnapStore {
    /// Creates a new `SnapStore` that operates in the given directory.
    ///
    /// The directory **does not need to exist** — it will be created on first write.
    ///
    /// # Arguments
    ///
    /// * `dir` — Directory to store snapshot files.
    pub fn new<P: AsRef<Path>>(dir: P) -> Self {
        let dir = dir.as_ref().to_path_buf();
        let snapshot_path = dir.join(SNAPSHOT_FILE);
        let metadata_path = dir.join(METADATA_FILE);
        Self {
            dir,
            snapshot_path,
            metadata_path,
        }
    }

    /// Atomically creates a new snapshot of the given state and sequence number.
    ///
    /// The snapshot is written to temporary files and then atomically renamed
    /// to the final names, ensuring:
    /// - No partial/corrupted snapshots are visible.
    /// - A reader always sees either the old snapshot or the new one — never an intermediate state.
    ///
    /// # Arguments
    ///
    /// * `state` — The serializable state to snapshot.
    /// * `seq` — Logical sequence number (e.g., last applied WAL index).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Serialization fails.
    /// - The snapshot directory cannot be created.
    /// - Disk I/O fails during write or rename.
    pub async fn create<S>(&self, state: &S, seq: u64) -> Result<()>
    where
        S: Serialize,
    {
        fs::create_dir_all(&self.dir).await?;

        // Write state to temp file
        let state_tmp = self.snapshot_path.with_extension("json.tmp");
        let state_json = serde_json::to_string_pretty(state)?;
        fs::write(&state_tmp, state_json)
            .await
            .map_err(|e| MiniSnapError::Io { source: e, path: state_tmp.clone() })?;

        // Write sequence to temp file
        let seq_tmp = self.metadata_path.with_extension("seq.tmp");
        fs::write(&seq_tmp, seq.to_string())
            .await
            .map_err(|e| MiniSnapError::Io { source: e, path: seq_tmp.clone() })?;

        // Atomic rename
        fs::rename(&state_tmp, &self.snapshot_path)
            .await
            .map_err(|e| MiniSnapError::Io { source: e, path: state_tmp })?;
        fs::rename(&seq_tmp, &self.metadata_path)
            .await
            .map_err(|e| MiniSnapError::Io { source: e, path: seq_tmp })?;

        Ok(())
    }

    /// Restores the latest snapshot and its sequence number.
    ///
    /// Reads both `snapshot.json` and `snapshot.seq` and ensures they exist.
    ///
    /// # Returns
    ///
    /// A tuple `(state, seq)` on success.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Either file is missing.
    /// - Deserialization fails.
    /// - Sequence file contains invalid number.
    pub async fn restore<S>(&self) -> Result<(S, u64)>
    where
        S: DeserializeOwned,
    {
        // Read state
        let state_json = fs::read_to_string(&self.snapshot_path)
            .await
            .map_err(|e| MiniSnapError::Io { source: e, path: self.snapshot_path.clone() })?;
        let state: S = serde_json::from_str(&state_json)?;

        // Read sequence
        let seq_str = fs::read_to_string(&self.metadata_path)
            .await
            .map_err(|e| MiniSnapError::Io { source: e, path: self.metadata_path.clone() })?;
        let seq = seq_str.trim().parse::<u64>()
            .map_err(|_| MiniSnapError::InvalidSequence)?;

        Ok((state, seq))
    }

    /// Returns the directory used for snapshot storage.
    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

//! Filesystem abstraction trait and implementations for cube-presets.
//!
//! `FsOps` is the trait that `PresetStore` uses for all filesystem calls,
//! enabling fault-injection and call-order verification in tests.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

// ─────────────────────────────────────────────────────────────────────────────
// FsCall — record of a single filesystem call (for testing)
// ─────────────────────────────────────────────────────────────────────────────

/// A recorded filesystem call made via `FsOps`. Used by `RecordingFsOps` to
/// assert call ordering (SDS §5.6).
#[derive(Debug, Clone)]
pub enum FsCall {
    /// Write `content` to a new temp file at `path`.
    Write(PathBuf, Vec<u8>),
    /// `fsync` the file at `path`.
    FsyncFile(PathBuf),
    /// Rename (atomic) from `from` to `to`.
    Rename { from: PathBuf, to: PathBuf },
    /// `fsync` the parent directory of `path` (path is the *directory*).
    FsyncParent(PathBuf),
    /// Remove a user preset file.
    RemoveFile(PathBuf),
    /// Create an empty tombstone file at `path`.
    CreateTombstone(PathBuf),
}

// ─────────────────────────────────────────────────────────────────────────────
// FsOps trait
// ─────────────────────────────────────────────────────────────────────────────

/// Abstraction over filesystem operations used by `PresetStore`.
///
/// The real implementation calls the actual OS; the `RecordingFsOps`
/// implementation records calls for test assertions.
pub trait FsOps: Send + Sync {
    /// Write bytes to a new temp file in the same directory as `target`.
    ///
    /// Returns the path of the created temp file.
    fn write_temp(&self, target: &Path, content: &[u8]) -> std::io::Result<PathBuf>;

    /// `fsync` a file.
    fn fsync_file(&self, path: &Path) -> std::io::Result<()>;

    /// Atomically rename `from` to `to`.
    fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()>;

    /// `fsync` the parent directory of `path`.
    ///
    /// `path` is the *target file* whose parent we want to fsync.
    fn fsync_parent(&self, target: &Path) -> std::io::Result<()>;

    /// Remove a file.
    fn remove_file(&self, path: &Path) -> std::io::Result<()>;

    /// Create an empty tombstone file at `path` (truncating if it exists).
    fn create_tombstone(&self, path: &Path) -> std::io::Result<()>;
}

// ─────────────────────────────────────────────────────────────────────────────
// RealFsOps — production implementation
// ─────────────────────────────────────────────────────────────────────────────

/// Real filesystem implementation of `FsOps`.
#[derive(Debug, Clone, Default)]
pub struct RealFsOps;

impl FsOps for RealFsOps {
    fn write_temp(&self, target: &Path, content: &[u8]) -> std::io::Result<PathBuf> {
        use std::io::Write;

        let dir = target
            .parent()
            .ok_or_else(|| std::io::Error::other("target has no parent"))?;

        // Generate a unique temp filename in the same directory.
        let rand_suffix = {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        };
        let tmp_name = format!(".tmp.{rand_suffix:08x}");
        let tmp_path = dir.join(&tmp_name);

        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp_path)?;
        f.write_all(content)?;

        Ok(tmp_path)
    }

    fn fsync_file(&self, path: &Path) -> std::io::Result<()> {
        let f = std::fs::OpenOptions::new().write(true).open(path)?;
        f.sync_all()
    }

    fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        std::fs::rename(from, to)
    }

    fn fsync_parent(&self, target: &Path) -> std::io::Result<()> {
        let dir = target
            .parent()
            .ok_or_else(|| std::io::Error::other("target has no parent"))?;
        let d = std::fs::File::open(dir)?;
        d.sync_all()
    }

    fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        std::fs::remove_file(path)
    }

    fn create_tombstone(&self, path: &Path) -> std::io::Result<()> {
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// RecordingFsOps — test implementation
// ─────────────────────────────────────────────────────────────────────────────

/// A `FsOps` implementation that:
/// 1. Records every call in `calls`.
/// 2. Optionally holds a `Barrier` after a particular call kind, enabling
///    cross-thread timing control for `flock` tests.
pub struct RecordingFsOps {
    /// The real filesystem impl used for actual I/O.
    inner: RealFsOps,
    /// Shared call log.
    pub calls: Arc<Mutex<Vec<FsCall>>>,
    /// If set, the `RecordingFsOps` will call `barrier.wait()` after the
    /// first call whose discriminant matches `barrier_after`.
    barrier: Option<(BarrierTrigger, Arc<std::sync::Barrier>)>,
}

/// Discriminant used to identify which call triggers the barrier.
#[derive(Clone)]
enum BarrierTrigger {
    FsyncFile,
}

impl RecordingFsOps {
    /// Create a `RecordingFsOps` with no barrier.
    pub fn new(_root: &Path) -> Self {
        Self {
            inner: RealFsOps,
            calls: Arc::new(Mutex::new(Vec::new())),
            barrier: None,
        }
    }

    /// Create a `RecordingFsOps` that waits at `barrier` after the first call
    /// matching `after`.  `after` must be a `FsCall` whose variant is used as
    /// a discriminant (only the variant, not payload, is matched).
    pub fn with_barrier_after(
        _root: &Path,
        after: FsCall,
        barrier: Arc<std::sync::Barrier>,
    ) -> Self {
        let trigger = match after {
            FsCall::FsyncFile(_) => BarrierTrigger::FsyncFile,
            _ => panic!("with_barrier_after: unsupported FsCall variant"),
        };
        Self {
            inner: RealFsOps,
            calls: Arc::new(Mutex::new(Vec::new())),
            barrier: Some((trigger, barrier)),
        }
    }

    fn record(&self, call: FsCall) {
        self.calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(call);
    }

    /// After recording a call, check whether the barrier should fire.
    fn maybe_barrier(&self, call: &FsCall) {
        if let Some((trigger, barrier)) = &self.barrier {
            let should_wait = matches!(
                (trigger, call),
                (BarrierTrigger::FsyncFile, FsCall::FsyncFile(_))
            );
            if should_wait {
                barrier.wait();
            }
        }
    }
}

impl FsOps for RecordingFsOps {
    fn write_temp(&self, target: &Path, content: &[u8]) -> std::io::Result<PathBuf> {
        let tmp_path = self.inner.write_temp(target, content)?;
        let call = FsCall::Write(tmp_path.clone(), content.to_vec());
        self.record(call.clone());
        self.maybe_barrier(&call);
        Ok(tmp_path)
    }

    fn fsync_file(&self, path: &Path) -> std::io::Result<()> {
        self.inner.fsync_file(path)?;
        let call = FsCall::FsyncFile(path.to_path_buf());
        self.record(call.clone());
        self.maybe_barrier(&call);
        Ok(())
    }

    fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        self.inner.rename(from, to)?;
        let call = FsCall::Rename {
            from: from.to_path_buf(),
            to: to.to_path_buf(),
        };
        self.record(call.clone());
        self.maybe_barrier(&call);
        Ok(())
    }

    fn fsync_parent(&self, target: &Path) -> std::io::Result<()> {
        self.inner.fsync_parent(target)?;
        let parent = target
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let call = FsCall::FsyncParent(parent);
        self.record(call.clone());
        self.maybe_barrier(&call);
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        self.inner.remove_file(path)?;
        let call = FsCall::RemoveFile(path.to_path_buf());
        self.record(call.clone());
        self.maybe_barrier(&call);
        Ok(())
    }

    fn create_tombstone(&self, path: &Path) -> std::io::Result<()> {
        self.inner.create_tombstone(path)?;
        let call = FsCall::CreateTombstone(path.to_path_buf());
        self.record(call.clone());
        self.maybe_barrier(&call);
        Ok(())
    }
}

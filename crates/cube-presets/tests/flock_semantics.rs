//! Advisory `flock(LOCK_EX)` semantics on `<user_root>/<app>/.lock`.
//!
//! - While `cubed` holds the lock around a save, an external observer using
//!   `LOCK_EX | LOCK_NB` on the same path gets `EWOULDBLOCK`.
//! - Once `cubed` releases the lock at the end of the operation, the same
//!   probe succeeds.

mod common;

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;

use cube_presets::{FsCall, PresetFile, PresetMeta, RecordingFsOps};
use cube_proto::ParamValue;
use nix::fcntl::{Flock, FlockArg};

fn preset_file() -> PresetFile {
    let mut params: BTreeMap<String, ParamValue> = BTreeMap::new();
    params.insert("speed".to_string(), ParamValue::Int(1));
    PresetFile {
        meta: PresetMeta {
            app: "x".to_string(),
            schema_version: 1,
            preset_name: "p".to_string(),
            author: None,
            description: None,
            created: chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0).unwrap(),
            version: "1.0".to_string(),
        },
        params,
    }
}

#[test]
fn flock_blocks_concurrent_saver() {
    let tmp = tempfile::tempdir().unwrap();
    let (system_root, user_root) = common::make_layout(tmp.path(), "x");
    let lock_path = user_root.join("x/.lock");

    // Mid-save barrier. RecordingFsOps holds the barrier *between*
    // FsyncFile and Rename so the lock is still held by `save`.
    let barrier = Arc::new(Barrier::new(2));
    let fs = RecordingFsOps::with_barrier_after(
        tmp.path(),
        FsCall::FsyncFile(std::path::PathBuf::new()),
        barrier.clone(),
    );

    let store = cube_presets::PresetStore::new(system_root, user_root.clone(), fs);
    let schema = common::schema_minimal("x", 1);

    // Spawn the save thread; it will park inside FsyncFile until we release.
    let save_handle = thread::spawn(move || {
        store
            .save("x", "p", &preset_file(), &schema)
            .expect("save must succeed");
    });

    // Wait long enough that the save has acquired the.lock and reached the
    // barrier. We need the lock file to exist before probing.
    let mut tries = 0;
    while !lock_path.exists() && tries < 500 {
        thread::sleep(Duration::from_millis(2));
        tries += 1;
    }
    assert!(lock_path.exists(), "expected .lock to be created by save()");
    // Tiny sleep so we are inside the flock-protected critical section.
    thread::sleep(Duration::from_millis(20));

    // Probe with LOCK_EX | LOCK_NB — must return EWOULDBLOCK while held.
    let probe = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock_path)
        .expect("open .lock for probe");
    let (probe, err) = Flock::lock(probe, FlockArg::LockExclusiveNonblock)
        .expect_err("expected flock to fail with EWOULDBLOCK while save holds the lock");
    assert_eq!(err, nix::errno::Errno::EWOULDBLOCK, "got errno {err:?}");
    drop(probe);

    // Release the barrier; save proceeds and unlocks.
    barrier.wait();
    save_handle.join().expect("save thread must succeed");

    // After release the probe must succeed.
    let probe2 = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock_path)
        .expect("re-open .lock");
    let locked = Flock::lock(probe2, FlockArg::LockExclusiveNonblock)
        .map_err(|(_, e)| e)
        .expect("flock must succeed after save released the lock");
    locked.unlock().map_err(|(_, e)| e).unwrap();
}

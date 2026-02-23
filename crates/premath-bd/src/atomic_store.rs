//! Lock-scoped atomic mutation helpers for JSONL issue memory.

use crate::jsonl::JsonlError;
use crate::{MemoryStore, MemoryStoreError};
use chrono::Utc;
use std::error::Error as StdError;
use std::ffi::OsString;
use std::fmt::{Display, Formatter};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

const LOCK_ACQUIRE_MAX_ATTEMPTS: usize = 256;
const LOCK_ACQUIRE_RETRY_DELAY: Duration = Duration::from_millis(2);

pub fn issue_lock_path(issues_path: &Path) -> PathBuf {
    let mut path: OsString = issues_path.as_os_str().to_os_string();
    path.push(".lock");
    PathBuf::from(path)
}

#[derive(Debug)]
pub enum AtomicStoreMutationError<E> {
    LockBusy { lock_path: String },
    LockIo { lock_path: String, message: String },
    Store(MemoryStoreError),
    Mutation(E),
}

impl<E> AtomicStoreMutationError<E> {
    fn lock_busy(lock_path: &Path) -> Self {
        Self::LockBusy {
            lock_path: lock_path.display().to_string(),
        }
    }

    fn lock_io(lock_path: &Path, message: impl Into<String>) -> Self {
        Self::LockIo {
            lock_path: lock_path.display().to_string(),
            message: message.into(),
        }
    }
}

impl<E: Display> Display for AtomicStoreMutationError<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LockBusy { lock_path } => write!(f, "issue-memory lock busy: {lock_path}"),
            Self::LockIo { lock_path, message } => {
                write!(
                    f,
                    "failed to acquire issue-memory lock {lock_path}: {message}"
                )
            }
            Self::Store(err) => write!(f, "{err}"),
            Self::Mutation(err) => write!(f, "{err}"),
        }
    }
}

impl<E> StdError for AtomicStoreMutationError<E> where
    E: Display + std::fmt::Debug + StdError + 'static
{
}

/// Execute one lock-scoped store mutation against an issues JSONL path.
///
/// The mutator returns `(value, changed)` where:
/// - `value` is returned to the caller
/// - `changed=true` persists the store to JSONL before lock release.
pub fn mutate_store_jsonl<T, E, F>(
    path: impl AsRef<Path>,
    mutator: F,
) -> Result<T, AtomicStoreMutationError<E>>
where
    F: FnOnce(&mut MemoryStore) -> Result<(T, bool), E>,
{
    let path = path.as_ref();
    let _guard = IssueFileLockGuard::acquire(path).map_err(|err| match err {
        AtomicStoreMutationError::LockBusy { lock_path } => {
            AtomicStoreMutationError::LockBusy { lock_path }
        }
        AtomicStoreMutationError::LockIo { lock_path, message } => {
            AtomicStoreMutationError::LockIo { lock_path, message }
        }
        AtomicStoreMutationError::Store(source) => AtomicStoreMutationError::Store(source),
        AtomicStoreMutationError::Mutation(unreachable) => match unreachable {},
    })?;

    let mut store = if path.exists() {
        validate_mutation_substrate(path).map_err(AtomicStoreMutationError::Store)?;
        MemoryStore::load_jsonl(path).map_err(AtomicStoreMutationError::Store)?
    } else {
        MemoryStore::default()
    };
    let (value, changed) = mutator(&mut store).map_err(AtomicStoreMutationError::Mutation)?;
    if changed {
        store
            .save_jsonl(path)
            .map_err(AtomicStoreMutationError::Store)?;
    }
    Ok(value)
}

struct IssueFileLockGuard {
    lock_path: PathBuf,
    _file: File,
}

impl IssueFileLockGuard {
    fn acquire(path: &Path) -> Result<Self, AtomicStoreMutationError<std::convert::Infallible>> {
        let lock_path = issue_lock_path(path);
        if let Some(parent) = lock_path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)
                .map_err(|e| AtomicStoreMutationError::lock_io(&lock_path, e.to_string()))?;
        }

        for attempt in 0..LOCK_ACQUIRE_MAX_ATTEMPTS {
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(mut file) => {
                    let _ = writeln!(
                        file,
                        "pid={}\nutc={}",
                        std::process::id(),
                        Utc::now().to_rfc3339()
                    );
                    return Ok(Self {
                        lock_path,
                        _file: file,
                    });
                }
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                    if attempt + 1 == LOCK_ACQUIRE_MAX_ATTEMPTS {
                        return Err(AtomicStoreMutationError::lock_busy(&lock_path));
                    }
                    thread::sleep(LOCK_ACQUIRE_RETRY_DELAY);
                }
                Err(err) => {
                    return Err(AtomicStoreMutationError::lock_io(
                        &lock_path,
                        err.to_string(),
                    ));
                }
            }
        }

        Err(AtomicStoreMutationError::lock_busy(&lock_path))
    }
}

impl Drop for IssueFileLockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.lock_path);
    }
}

fn validate_mutation_substrate(path: &Path) -> Result<(), MemoryStoreError> {
    let bytes = fs::read(path).map_err(|e| {
        MemoryStoreError::Jsonl(JsonlError::Io(0, format!("{}: {e}", path.display())))
    })?;

    if bytes.contains(&0) {
        return Err(MemoryStoreError::Jsonl(JsonlError::Corrupt(format!(
            "{}: contains NUL byte(s)",
            path.display()
        ))));
    }
    if std::str::from_utf8(&bytes).is_err() {
        return Err(MemoryStoreError::Jsonl(JsonlError::Corrupt(format!(
            "{}: contains non-UTF-8 byte sequence(s)",
            path.display()
        ))));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Issue;
    use std::convert::Infallible;
    use std::sync::{Arc, Barrier};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_issues_path(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("premath-atomic-{prefix}-{unique}"));
        fs::create_dir_all(&root).expect("temp dir should be created");
        root.join("issues.jsonl")
    }

    #[test]
    fn mutate_store_jsonl_contention_preserves_jsonl_integrity() {
        let path = temp_issues_path("contention");
        let workers = 8;
        let barrier = Arc::new(Barrier::new(workers + 1));
        let mut handles = Vec::new();

        for idx in 0..workers {
            let path = path.clone();
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                let issue_id = format!("bd-{idx}");
                barrier.wait();
                mutate_store_jsonl::<(), Infallible, _>(&path, |store| {
                    let mut issue = Issue::new(&issue_id, format!("Issue {issue_id}"));
                    issue.set_status("open".to_string());
                    store.upsert_issue(issue);
                    Ok(((), true))
                })
            }));
        }
        barrier.wait();

        for handle in handles {
            let result = handle.join().expect("worker should join");
            assert!(result.is_ok(), "worker mutation should succeed: {result:?}");
        }

        let bytes = fs::read(&path).expect("issues jsonl should exist");
        assert!(!bytes.contains(&0), "jsonl should never contain NUL bytes");

        let store = MemoryStore::load_jsonl(&path).expect("store should reload");
        assert_eq!(store.len(), workers);
        for idx in 0..workers {
            assert!(
                store.issue(&format!("bd-{idx}")).is_some(),
                "expected issue bd-{idx} to exist"
            );
        }
    }

    #[test]
    fn mutate_store_jsonl_reports_lock_busy_without_modifying_store() {
        let path = temp_issues_path("lock-busy");
        let mut initial = Issue::new("bd-1", "Issue 1");
        initial.set_status("open".to_string());
        let store = MemoryStore::from_issues(vec![initial]).expect("store should build");
        store.save_jsonl(&path).expect("store should save");

        let lock_path = issue_lock_path(&path);
        fs::write(&lock_path, "busy\n").expect("lock should be created");

        let result = mutate_store_jsonl::<(), Infallible, _>(&path, |store| {
            let issue = store
                .issue_mut("bd-1")
                .expect("issue should be present for mutation");
            issue.set_status("closed".to_string());
            Ok(((), true))
        });

        match result {
            Err(AtomicStoreMutationError::LockBusy {
                lock_path: reported,
            }) => {
                assert_eq!(reported, lock_path.display().to_string());
            }
            other => panic!("expected lock busy error, got {other:?}"),
        }

        let reloaded = MemoryStore::load_jsonl(&path).expect("store should reload");
        assert_eq!(
            reloaded
                .issue("bd-1")
                .expect("issue should be present")
                .status,
            "open"
        );

        let _ = fs::remove_file(lock_path);
    }

    #[test]
    fn mutate_store_jsonl_rejects_corrupt_substrate_before_mutation() {
        let path = temp_issues_path("corrupt");
        fs::write(
            &path,
            b"{\"id\":\"bd-1\",\"title\":\"Issue 1\",\"status\":\"open\"}\n\0tail",
        )
        .expect("fixture should write");

        let result = mutate_store_jsonl::<(), Infallible, _>(&path, |_store| Ok(((), true)));

        match result {
            Err(AtomicStoreMutationError::Store(MemoryStoreError::Jsonl(JsonlError::Corrupt(
                message,
            )))) => {
                assert!(message.contains("contains NUL"));
            }
            other => panic!("expected corrupt substrate rejection, got {other:?}"),
        }

        let bytes = fs::read(&path).expect("fixture should remain untouched");
        assert!(bytes.contains(&0));
    }
}

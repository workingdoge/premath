//! Lock-scoped atomic mutation helpers for JSONL issue memory.

use crate::{MemoryStore, MemoryStoreError};
use chrono::Utc;
use std::error::Error as StdError;
use std::ffi::OsString;
use std::fmt::{Display, Formatter};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

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

    let mut store = MemoryStore::load_jsonl(path).map_err(AtomicStoreMutationError::Store)?;
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
                Ok(Self {
                    lock_path,
                    _file: file,
                })
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                Err(AtomicStoreMutationError::lock_busy(&lock_path))
            }
            Err(err) => Err(AtomicStoreMutationError::lock_io(
                &lock_path,
                err.to_string(),
            )),
        }
    }
}

impl Drop for IssueFileLockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.lock_path);
    }
}

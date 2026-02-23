//! JSONL storage: one line per issue.
//!
//! The portable interchange format. Every issue is a single JSON line.
//! JJ versions this file. SurrealDB hydrates from it.

use crate::issue::Issue;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Read issues from a JSONL reader.
pub fn read_issues(reader: impl BufRead) -> Result<Vec<Issue>, JsonlError> {
    let mut issues = Vec::new();
    for (line_no, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| JsonlError::Io(line_no + 1, e.to_string()))?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let issue: Issue = serde_json::from_str(trimmed)
            .map_err(|e| JsonlError::Parse(line_no + 1, e.to_string()))?;
        issues.push(issue);
    }
    Ok(issues)
}

/// Write issues to a JSONL writer.
pub fn write_issues(writer: &mut impl Write, issues: &[Issue]) -> Result<(), JsonlError> {
    for issue in issues {
        let line =
            serde_json::to_string(issue).map_err(|e| JsonlError::Serialize(e.to_string()))?;
        writeln!(writer, "{line}").map_err(|e| JsonlError::Io(0, e.to_string()))?;
    }
    Ok(())
}

/// Read issues from a JSONL file path.
pub fn read_issues_from_path(path: impl AsRef<Path>) -> Result<Vec<Issue>, JsonlError> {
    let path = path.as_ref();
    let bytes =
        fs::read(path).map_err(|e| JsonlError::Io(0, format!("{}: {e}", path.display())))?;
    validate_substrate_bytes(path, &bytes)?;
    let reader = BufReader::new(bytes.as_slice());
    read_issues(reader)
}

/// Write issues to a JSONL file path.
pub fn write_issues_to_path(path: impl AsRef<Path>, issues: &[Issue]) -> Result<(), JsonlError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|e| JsonlError::Io(0, format!("{parent:?}: {e}")))?;
    }

    let tmp_path = tmp_write_path(path);
    let write_result = (|| -> Result<(), JsonlError> {
        let file = File::create(&tmp_path)
            .map_err(|e| JsonlError::Io(0, format!("{}: {e}", tmp_path.display())))?;
        let mut writer = BufWriter::new(file);
        write_issues(&mut writer, issues)?;
        writer
            .flush()
            .map_err(|e| JsonlError::Io(0, format!("{}: {e}", tmp_path.display())))?;
        let file = writer
            .into_inner()
            .map_err(|e| JsonlError::Io(0, format!("{}: {e}", tmp_path.display())))?;
        file.sync_all()
            .map_err(|e| JsonlError::Io(0, format!("{}: {e}", tmp_path.display())))?;
        Ok(())
    })();

    if let Err(error) = write_result {
        let _ = fs::remove_file(&tmp_path);
        return Err(error);
    }

    fs::rename(&tmp_path, path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        JsonlError::Io(
            0,
            format!("{} -> {}: {e}", tmp_path.display(), path.display()),
        )
    })?;

    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        let dir = File::open(parent)
            .map_err(|e| JsonlError::Io(0, format!("{}: {e}", parent.display())))?;
        dir.sync_all()
            .map_err(|e| JsonlError::Io(0, format!("{}: {e}", parent.display())))?;
    }

    Ok(())
}

fn tmp_write_path(path: &Path) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut tmp: OsString = path.as_os_str().to_os_string();
    tmp.push(format!(".tmp.{}.{}", std::process::id(), unique));
    PathBuf::from(tmp)
}

fn validate_substrate_bytes(path: &Path, bytes: &[u8]) -> Result<(), JsonlError> {
    if bytes.contains(&0) {
        return Err(JsonlError::Corrupt(format!(
            "{}: contains NUL byte(s)",
            path.display()
        )));
    }
    if std::str::from_utf8(bytes).is_err() {
        return Err(JsonlError::Corrupt(format!(
            "{}: contains non-UTF-8 byte sequence(s)",
            path.display()
        )));
    }
    Ok(())
}

/// Errors from JSONL operations.
#[derive(Debug, thiserror::Error)]
pub enum JsonlError {
    #[error("line {0}: I/O error: {1}")]
    Io(usize, String),

    #[error("line {0}: parse error: {1}")]
    Parse(usize, String),

    #[error("serialization error: {0}")]
    Serialize(String),

    #[error("corrupted substrate: {0}")]
    Corrupt(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_path(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "premath-jsonl-{prefix}-{}-{unique}.jsonl",
            std::process::id()
        ))
    }

    #[test]
    fn read_issues_from_path_rejects_nul_payload() {
        let path = temp_path("nul");
        fs::write(
            &path,
            b"{\"id\":\"bd-1\",\"title\":\"Issue\",\"status\":\"open\"}\n\0garbage",
        )
        .expect("fixture should write");

        let result = read_issues_from_path(&path);
        match result {
            Err(JsonlError::Corrupt(message)) => {
                assert!(message.contains("contains NUL"));
            }
            other => panic!("expected corrupt substrate error, got {other:?}"),
        }

        let _ = fs::remove_file(path);
    }

    #[test]
    fn read_issues_from_path_rejects_non_utf8_payload() {
        let path = temp_path("non-utf8");
        fs::write(&path, [0xff, 0xfe, 0xfd]).expect("fixture should write");

        let result = read_issues_from_path(&path);
        match result {
            Err(JsonlError::Corrupt(message)) => {
                assert!(message.contains("non-UTF-8"));
            }
            other => panic!("expected corrupt substrate error, got {other:?}"),
        }

        let _ = fs::remove_file(path);
    }

    #[test]
    fn write_issues_to_path_replaces_file_atomically() {
        let path = temp_path("atomic-write");
        let mut first = Issue::new("bd-1", "First issue");
        first.set_status("open".to_string());
        write_issues_to_path(&path, &[first]).expect("first write should succeed");

        let mut second = Issue::new("bd-2", "Second issue");
        second.set_status("closed".to_string());
        write_issues_to_path(&path, &[second]).expect("second write should succeed");

        let lines = fs::read_to_string(&path).expect("jsonl should exist");
        assert!(!lines.contains("bd-1"));
        assert!(lines.contains("bd-2"));

        let _ = fs::remove_file(path);
    }
}

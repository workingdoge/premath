//! JSONL storage: one line per issue.
//!
//! The portable interchange format. Every issue is a single JSON line.
//! JJ versions this file. SurrealDB hydrates from it.

use crate::issue::Issue;
use std::fs::File;
use std::io::{BufRead, Write};
use std::path::Path;

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
    let file = File::open(path.as_ref())
        .map_err(|e| JsonlError::Io(0, format!("{}: {e}", path.as_ref().display())))?;
    let reader = std::io::BufReader::new(file);
    read_issues(reader)
}

/// Write issues to a JSONL file path.
pub fn write_issues_to_path(path: impl AsRef<Path>, issues: &[Issue]) -> Result<(), JsonlError> {
    let mut file = File::create(path.as_ref())
        .map_err(|e| JsonlError::Io(0, format!("{}: {e}", path.as_ref().display())))?;
    write_issues(&mut file, issues)
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
}

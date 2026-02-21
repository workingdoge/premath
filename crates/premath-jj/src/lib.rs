//! JJ adapter for repository snapshots and status.
//!
//! This crate is intentionally thin: it shells out to `jj` for operational
//! metadata and keeps no opinionated orchestration policy.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Errors from interacting with a JJ repository.
#[derive(Debug, thiserror::Error)]
pub enum JjError {
    #[error("jj executable is not available in PATH")]
    NotInstalled,

    #[error("jj command failed: jj {args} ({message})")]
    CommandFailed { args: String, message: String },

    #[error("unable to parse jj output: {0}")]
    Parse(String),
}

/// Snapshot of a JJ workspace at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JjSnapshot {
    pub repo_root: PathBuf,
    pub change_id: String,
    pub status: String,
}

/// Thin client around the `jj` CLI.
#[derive(Debug, Clone)]
pub struct JjClient {
    repo_root: PathBuf,
}

impl JjClient {
    /// Returns true if `jj` is available in PATH.
    pub fn is_available() -> bool {
        Command::new("jj")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Discover a JJ workspace from `path` by resolving `jj root`.
    pub fn discover(path: impl AsRef<Path>) -> Result<Self, JjError> {
        let stdout = run_jj(path.as_ref(), &["root"])?;
        let root = first_nonempty_line(&stdout)
            .ok_or_else(|| JjError::Parse("jj root returned empty output".to_string()))?;
        Ok(Self {
            repo_root: PathBuf::from(root),
        })
    }

    /// Filesystem path to the detected JJ repository root.
    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    /// Current change ID at `@`.
    pub fn current_change_id(&self) -> Result<String, JjError> {
        let stdout = run_jj(
            &self.repo_root,
            &["log", "-r", "@", "--no-graph", "-T", "change_id ++ \"\\n\""],
        )?;
        first_nonempty_line(&stdout)
            .map(ToOwned::to_owned)
            .ok_or_else(|| JjError::Parse("failed to parse change id from jj log".to_string()))
    }

    /// Human-readable status output from `jj status`.
    pub fn status(&self) -> Result<String, JjError> {
        run_jj(&self.repo_root, &["status"])
    }

    /// Capture a snapshot containing root, change id, and status text.
    pub fn snapshot(&self) -> Result<JjSnapshot, JjError> {
        Ok(JjSnapshot {
            repo_root: self.repo_root.clone(),
            change_id: self.current_change_id()?,
            status: self.status()?,
        })
    }
}

fn run_jj(cwd: &Path, args: &[&str]) -> Result<String, JjError> {
    let output = Command::new("jj")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                JjError::NotInstalled
            } else {
                JjError::CommandFailed {
                    args: args.join(" "),
                    message: err.to_string(),
                }
            }
        })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            "unknown error".to_string()
        } else {
            stderr
        };
        Err(JjError::CommandFailed {
            args: args.join(" "),
            message,
        })
    }
}

fn first_nonempty_line(input: &str) -> Option<&str> {
    input.lines().map(str::trim).find(|line| !line.is_empty())
}

#[cfg(test)]
mod tests {
    use super::first_nonempty_line;

    #[test]
    fn first_nonempty_line_finds_trimmed_line() {
        let s = "\n\n  abc123  \n";
        assert_eq!(first_nonempty_line(s), Some("abc123"));
    }

    #[test]
    fn first_nonempty_line_none_for_blank_input() {
        assert_eq!(first_nonempty_line(" \n\t\n"), None);
    }
}

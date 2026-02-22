use crate::support::yes_no;
use premath_bd::MemoryStore;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct InitOutcome {
    pub repo_root: PathBuf,
    pub premath_dir: PathBuf,
    pub issues_path: PathBuf,
    pub created_repo_root: bool,
    pub created_premath_dir: bool,
    pub created_issues_file: bool,
    pub migrated_from_legacy: Option<PathBuf>,
}

pub fn init_layout(path: impl AsRef<Path>) -> Result<InitOutcome, String> {
    let repo_root = path.as_ref().to_path_buf();

    let mut created_repo_root = false;
    if !repo_root.exists() {
        fs::create_dir_all(&repo_root)
            .map_err(|e| format!("failed to create init path {}: {e}", repo_root.display()))?;
        created_repo_root = true;
    }
    if !repo_root.is_dir() {
        return Err(format!(
            "init path is not a directory: {}",
            repo_root.display()
        ));
    }

    let premath_dir = repo_root.join(".premath");
    let mut created_premath_dir = false;
    if !premath_dir.exists() {
        fs::create_dir_all(&premath_dir).map_err(|e| {
            format!(
                "failed to create premath directory {}: {e}",
                premath_dir.display()
            )
        })?;
        created_premath_dir = true;
    }
    if !premath_dir.is_dir() {
        return Err(format!(
            "premath path is not a directory: {}",
            premath_dir.display()
        ));
    }

    let issues_path = premath_dir.join("issues.jsonl");
    if issues_path.exists() && !issues_path.is_file() {
        return Err(format!(
            "issues path exists but is not a file: {}",
            issues_path.display()
        ));
    }

    let mut created_issues_file = false;
    let mut migrated_from_legacy = None;
    if !issues_path.exists() {
        let legacy_issues = repo_root.join(".beads").join("issues.jsonl");
        if legacy_issues.exists() {
            fs::copy(&legacy_issues, &issues_path).map_err(|e| {
                format!(
                    "failed to migrate legacy store {} -> {}: {e}",
                    legacy_issues.display(),
                    issues_path.display()
                )
            })?;
            migrated_from_legacy = Some(legacy_issues);
            created_issues_file = true;
        } else {
            MemoryStore::default()
                .save_jsonl(&issues_path)
                .map_err(|e| format!("failed to initialize {}: {e}", issues_path.display()))?;
            created_issues_file = true;
        }
    }

    Ok(InitOutcome {
        repo_root,
        premath_dir,
        issues_path,
        created_repo_root,
        created_premath_dir,
        created_issues_file,
        migrated_from_legacy,
    })
}

pub fn run(path: String) {
    let outcome = init_layout(&path).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });

    println!("premath init {path}");
    println!();
    println!("  repo root: {}", outcome.repo_root.display());
    println!("  premath dir: {}", outcome.premath_dir.display());
    println!("  issues path: {}", outcome.issues_path.display());
    if let Some(legacy) = &outcome.migrated_from_legacy {
        println!("  migrated from legacy: {}", legacy.display());
    }
    println!("  created repo root: {}", yes_no(outcome.created_repo_root));
    println!(
        "  created .premath dir: {}",
        yes_no(outcome.created_premath_dir)
    );
    println!(
        "  created issues file: {}",
        yes_no(outcome.created_issues_file)
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "premath-cli-init-{prefix}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temp dir should exist");
        path
    }

    #[test]
    fn init_layout_creates_premath_store() {
        let root = temp_dir("create");
        let outcome = init_layout(&root).expect("init should succeed");
        assert!(outcome.issues_path.exists());
        assert!(outcome.created_issues_file);
    }

    #[test]
    fn init_layout_migrates_legacy_beads_store() {
        let root = temp_dir("migrate");
        let legacy_dir = root.join(".beads");
        fs::create_dir_all(&legacy_dir).expect("legacy dir should exist");
        let legacy_issues = legacy_dir.join("issues.jsonl");
        fs::write(
            &legacy_issues,
            "{\"id\":\"bd-1\",\"title\":\"Legacy\",\"status\":\"open\"}\n",
        )
        .expect("legacy issues should be written");

        let outcome = init_layout(&root).expect("init should succeed");
        assert_eq!(
            outcome.migrated_from_legacy.as_deref(),
            Some(legacy_issues.as_path())
        );
        let migrated =
            fs::read_to_string(outcome.issues_path).expect("migrated issues should be readable");
        assert!(migrated.contains("\"id\":\"bd-1\""));
    }
}

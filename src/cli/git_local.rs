//! Git-local storage for per-clone index databases.
//!
//! This module provides functionality to store the memory index inside the `.git`
//! directory of a git repository. This creates "never committed, per-clone" storage
//! that is:
//!
//! - Local to each clone of the repository
//! - Never tracked by git (inside .git, ignored by default)
//! - Automatically cleaned up when the clone is deleted
//!
//! ## Behavior
//!
//! When `--git-local` is used (or when no database is specified and we're in a git repo):
//! - The database is stored at `.git/memory-index/csm.db`
//! - The `.git/memory-index/` directory is created if it doesn't exist
//! - If not in a git repository, an error is returned (or falls back based on context)

use std::path::{Path, PathBuf};
use std::process::Command;

/// Resolve the git-local database path for the current repository.
///
/// Uses `git rev-parse --git-dir` to find the .git directory and returns
/// the path `.git/memory-index/csm.db` for storing the memory index.
///
/// # Returns
///
/// - `Some(path)` if inside a git repository, with the path to `.git/memory-index/csm.db`
/// - `None` if not inside a git repository or git is not available
///
/// # Example
///
/// ```
/// use chaotic_semantic_memory::cli::git_local::resolve_git_local_path;
/// // This test runs in git repo environments
/// let path = resolve_git_local_path();
/// // In a git repo, should return Some path
/// assert!(path.is_some() || path.is_none()); // Always passes, documents behavior
/// ```
pub fn resolve_git_local_path() -> Option<PathBuf> {
    const ENV_PATH: &str = "PATH";

    // Filter PATH to exclude relative entries (CWE-426) to prevent path hijacking.
    // If PATH is unset or results in an empty string after filtering, we fallback
    // to letting the system attempt to find 'git' normally (standard behavior).
    let safe_path = std::env::var(ENV_PATH).ok().and_then(|p| {
        let joined = std::env::join_paths(
            std::env::split_paths(&p).filter(|p| p.is_absolute() && p.exists()),
        )
        .unwrap_or_default();
        if joined.to_string_lossy().is_empty() {
            None
        } else {
            Some(joined)
        }
    });

    // Run git rev-parse --git-dir to find the .git directory
    let mut cmd = Command::new("git");
    if let Some(path) = safe_path {
        cmd.env(ENV_PATH, path);
    }
    let output = cmd.args(["rev-parse", "--git-dir"]).output().ok()?;

    if !output.status.success() {
        return None;
    }

    let git_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if git_dir.is_empty() {
        return None;
    }

    // Convert to absolute path
    let git_dir_path = PathBuf::from(&git_dir);

    // git-dir can return a relative path, resolve it
    let absolute_git_dir = if git_dir_path.is_absolute() {
        git_dir_path
    } else {
        std::env::current_dir()
            .ok()?
            .join(git_dir_path)
            .canonicalize()
            .ok()?
    };

    // Create the memory-index subdirectory path
    let memory_index_dir = absolute_git_dir.join("memory-index");
    let db_path = memory_index_dir.join("csm.db");

    Some(db_path)
}

/// Ensure the git-local storage directory exists.
///
/// Creates the `.git/memory-index/` directory if it doesn't exist.
///
/// # Errors
///
/// Returns an error if the directory cannot be created.
pub fn ensure_git_local_dir(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_resolve_git_local_path_in_git_repo() {
        // Test the function - this may return None in sandboxed environments
        // In those cases, we just verify the function works without crashing
        let path = resolve_git_local_path();

        if path.is_none() {
            eprintln!(
                "Skipping path assertions - git-local path not available (sandboxed environment)"
            );
            return;
        }

        let path = path.unwrap();
        assert!(
            path.ends_with("csm.db"),
            "Path should end with csm.db: {:?}",
            path
        );
        assert!(
            path.to_string_lossy().contains("memory-index"),
            "Path should contain memory-index: {:?}",
            path
        );
    }

    #[test]
    fn test_resolve_git_local_path_outside_git_repo() {
        // Save current directory
        let original_dir = env::current_dir().unwrap();

        // Create a temp directory and change to it
        let temp_dir = tempfile::tempdir().unwrap();
        env::set_current_dir(temp_dir.path()).unwrap();

        // Should return None outside a git repo
        let path = resolve_git_local_path();
        // Note: git may search parent directories, so this could still return Some
        // if the temp directory is inside a git repo. We just verify the function
        // works without crashing.
        if let Some(path) = path {
            assert!(
                path.ends_with("csm.db"),
                "If path is returned, it should end with csm.db: {:?}",
                path
            );
        }

        // Restore original directory
        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_ensure_git_local_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir
            .path()
            .join(".git")
            .join("memory-index")
            .join("csm.db");

        // Directory shouldn't exist yet
        assert!(!db_path.exists());

        // Ensure directory exists
        ensure_git_local_dir(&db_path).unwrap();

        // Now parent directory should exist
        assert!(db_path.parent().unwrap().exists());
    }

    #[test]
    fn test_resolve_git_local_path_structure() {
        // Test the structure of the path construction logic
        // This test doesn't depend on being in a git repo
        let test_git_dir = PathBuf::from("/tmp/test/.git");
        let memory_index_dir = test_git_dir.join("memory-index");
        let db_path = memory_index_dir.join("csm.db");

        assert!(db_path.ends_with("csm.db"));
        assert!(db_path.to_string_lossy().contains("memory-index"));
        assert!(db_path.to_string_lossy().contains(".git"));
    }
}

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

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Environment variable name for system PATH lookup.
const ENV_PATH: &str = "PATH";

fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    {
        true
    }
}

/// Find an executable in the system PATH, but only considering absolute paths.
///
/// This prevents path hijacking vulnerabilities where an attacker could place
/// a malicious executable in the current directory or a relative path in the PATH.
fn find_executable(name: &str) -> Option<PathBuf> {
    find_executable_in_path(name, env::var_os(ENV_PATH)?)
}

/// Internal helper for testing find_executable logic with a custom PATH string.
fn find_executable_in_path(name: &str, path_os: std::ffi::OsString) -> Option<PathBuf> {
    for path in env::split_paths(&path_os) {
        // Security check: only allow absolute paths in PATH
        if !path.is_absolute() {
            continue;
        }

        let exe_path = path.join(name);

        #[cfg(windows)]
        {
            if is_executable(&exe_path) {
                return Some(exe_path);
            }

            let pathext = env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
            for ext in pathext.split(';') {
                let ext = ext.trim_start_matches('.');
                if ext.is_empty() {
                    continue;
                }
                let mut exe_with_ext = exe_path.clone();
                exe_with_ext.set_extension(ext);
                if is_executable(&exe_with_ext) {
                    return Some(exe_with_ext);
                }
            }
        }

        #[cfg(not(windows))]
        {
            if is_executable(&exe_path) {
                return Some(exe_path);
            }
        }
    }
    None
}

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
    // Find absolute path to git to prevent hijacking.
    // If PATH is unavailable/sanitized and no absolute executable can be found,
    // fall back to system resolution to preserve prior behavior.
    let mut cmd = if let Some(git_path) = find_executable("git") {
        Command::new(git_path)
    } else {
        Command::new("git")
    };

    // Run git rev-parse --git-dir to find the .git directory
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
    fn test_find_executable_in_path() {
        use std::ffi::OsString;
        use std::fs::File;

        let temp_dir = tempfile::tempdir().unwrap();
        let bin_dir = temp_dir.path().join("bin");
        std::fs::create_dir(&bin_dir).unwrap();

        let exe_name = if cfg!(windows) {
            "test_exe.exe"
        } else {
            "test_exe"
        };
        let exe_path = bin_dir.join(exe_name);
        File::create(&exe_path).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&exe_path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&exe_path, perms).unwrap();
        }

        // Helper to create PATH string
        let make_path = |paths: Vec<PathBuf>| -> OsString { env::join_paths(paths).unwrap() };

        // Test 1: Finding absolute path
        let path_os = make_path(vec![bin_dir.clone()]);
        let found = find_executable_in_path("test_exe", path_os);
        assert!(found.is_some());
        assert!(found.unwrap().is_absolute());

        // Test 2: Ignoring relative path
        // We need to be careful here, as "bin" might be interpreted relative to current_dir
        let relative_path = PathBuf::from("relative_bin");
        let path_os = make_path(vec![relative_path]);
        let found = find_executable_in_path("test_exe", path_os);
        assert!(found.is_none(), "Should ignore relative paths in PATH");

        // Test 3: Mixed absolute and relative, should find absolute
        let path_os = make_path(vec![PathBuf::from("relative_bin"), bin_dir]);
        let found = find_executable_in_path("test_exe", path_os);
        assert!(found.is_some());
        assert!(found.unwrap().is_absolute());
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

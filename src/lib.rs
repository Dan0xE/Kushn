use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::{env, fs, io};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Serialize, Deserialize)]
/// Represents a hashed file entry produced by [`process_file`] or [`process_directory`].
///
/// The `path` field always contains a relative path (from the directory that was
/// processed) and `hash` stores the lowercase hexadecimal SHA-256 digest.
pub struct FileHash {
    pub path: String,
    pub hash: String,
}

#[derive(Debug, Error)]
pub enum KushnError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Invalid glob pattern: {0}")]
    GlobPattern(#[from] glob::PatternError),
    #[error("Directory traversal error: {0}")]
    WalkDir(#[from] walkdir::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type KushnResult<T> = Result<T, KushnError>;

fn build_file_ignore_patterns(ignore: &[String]) -> Result<Vec<glob::Pattern>, glob::PatternError> {
    ignore
        .iter()
        .map(|pattern| {
            let normalized = pattern.replace('\\', "/");
            glob::Pattern::new(&format!("**/{}", normalized))
        })
        .collect()
}

/// Computes the SHA-256 digest for the file at `file_path`.
///
/// # Errors
/// Returns [`KushnError::Io`] if the file cannot be opened or read.
///
/// # Examples
/// ```
/// use kushn::{calculate_file_hash, KushnResult};
/// use std::io::Write;
/// use tempfile::NamedTempFile;
///
/// # fn main() -> KushnResult<()> {
/// let mut file = NamedTempFile::new()?;
/// write!(file, "kushn")?;
///
/// let digest = calculate_file_hash(file.path())?;
/// assert_eq!(digest.len(), 64);
/// # Ok(())
/// # }
/// ```
pub fn calculate_file_hash<P: AsRef<Path>>(file_path: P) -> KushnResult<String> {
    let mut file = fs::File::open(file_path)?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)?;
    let hash_result = hasher.finalize();
    Ok(format!("{:x}", hash_result))
}

/// Processes a single file and returns its hash unless it matches an ignore pattern.
///
/// This helper strips the current working directory from the final entry path so
/// downstream consumers receive relative paths.
///
/// # Errors
/// * [`KushnError::Io`] if the file cannot be read or the current directory is unavailable.
/// * [`KushnError::GlobPattern`] if an ignore pattern fails to compile.
///
/// # Examples
/// ```
/// use kushn::{process_file, KushnResult};
/// use std::env;
/// use std::fs;
/// use tempfile::tempdir;
///
/// # fn main() -> KushnResult<()> {
/// let dir = tempdir()?;
/// let file = dir.path().join("example.txt");
/// fs::write(&file, "example")?;
///
/// let original = env::current_dir()?;
/// env::set_current_dir(dir.path())?;
///
/// let entry = process_file("example.txt", &[])?.expect("file should be hashed");
/// assert_eq!(entry.path, "example.txt");
///
/// env::set_current_dir(original)?;
/// # Ok(())
/// # }
/// ```
pub fn process_file<P: AsRef<Path>>(
    file_path: P,
    ignore: &[String],
) -> KushnResult<Option<FileHash>> {
    let file_path = file_path.as_ref();
    let base_dir = env::current_dir()?;
    let relative_path = file_path.strip_prefix(&base_dir).unwrap_or(file_path);

    let ignore_patterns = build_file_ignore_patterns(ignore)?;

    let match_options = glob::MatchOptions::new();
    if ignore_patterns
        .iter()
        .any(|pattern| pattern.matches_path_with(relative_path, match_options))
    {
        return Ok(None);
    }

    let hash = calculate_file_hash(file_path)?;
    let path_string = relative_path.to_string_lossy().into_owned();
    Ok(Some(FileHash {
        path: path_string,
        hash,
    }))
}

/// Recursively walks a directory and returns hashed entries that are not ignored.
///
/// Directories or files matching any pattern in `ignore` (considered relative to the
/// provided directory) are skipped. Symlinks are followed
///
/// # Errors
/// * [`KushnError::WalkDir`] if a directory entry cannot be read.
/// * [`KushnError::GlobPattern`] if an ignore pattern is invalid.
/// * Any error bubbled up from [`process_file`].
///
/// # Examples
/// ```
/// use kushn::{process_directory, KushnResult};
/// use std::fs;
/// use tempfile::tempdir;
///
/// # fn main() -> KushnResult<()> {
/// let dir = tempdir()?;
/// fs::write(dir.path().join("keep.txt"), "contents")?;
/// fs::create_dir(dir.path().join("ignored"))?;
/// fs::write(dir.path().join("ignored/skip.txt"), "ignored")?;
///
/// let entries = process_directory(dir.path(), &["ignored".into()])?;
/// assert_eq!(entries.len(), 1);
/// assert_eq!(entries[0].path, "keep.txt");
/// # Ok(())
/// # }
/// ```
#[inline]
pub fn process_directory<P: AsRef<Path>>(
    directory_path: P,
    ignore: &[String],
) -> KushnResult<Vec<FileHash>> {
    let directory_path = directory_path.as_ref();
    let mut results = Vec::new();

    let directory_ignore_patterns = ignore
        .iter()
        .map(|pattern| {
            let normalized = pattern.replace('\\', "/");
            glob::Pattern::new(&format!("{}/**", normalized))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let file_ignore_patterns = build_file_ignore_patterns(ignore)?;

    let match_options = glob::MatchOptions::new();

    for entry in WalkDir::new(directory_path).follow_links(true) {
        let entry = entry?;
        let path = entry.path();
        let relative_path = path.strip_prefix(directory_path).unwrap_or(path);

        if entry.file_type().is_dir()
            && directory_ignore_patterns
                .iter()
                .any(|pattern| pattern.matches_path_with(relative_path, match_options))
        {
            continue;
        }

        if entry.file_type().is_file() {
            if directory_ignore_patterns
                .iter()
                .any(|pattern| pattern.matches_path_with(relative_path, match_options))
            {
                continue;
            }

            if file_ignore_patterns
                .iter()
                .any(|pattern| pattern.matches_path_with(relative_path, match_options))
            {
                continue;
            }

            let normalized_relative = relative_path
                .to_string_lossy()
                .replace('\\', "/");
            let hash = calculate_file_hash(path)?;
            results.push(FileHash {
                path: normalized_relative,
                hash,
            });
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::{Mutex, MutexGuard, OnceLock};
    use tempfile::tempdir;

    static ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

    fn env_lock() -> &'static Mutex<()> {
        ENV_MUTEX.get_or_init(|| Mutex::new(()))
    }

    fn lock_env_guard() -> MutexGuard<'static, ()> {
        env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    struct WorkingDirGuard {
        original: PathBuf,
    }

    impl WorkingDirGuard {
        fn set(path: &Path) -> KushnResult<Self> {
            let original = env::current_dir()?;
            let canonical = fs::canonicalize(path)?;
            env::set_current_dir(canonical)?;
            Ok(Self { original })
        }
    }

    impl Drop for WorkingDirGuard {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.original);
        }
    }

    fn with_working_dir<F, R>(dir: &Path, func: F) -> KushnResult<R>
    where
        F: FnOnce() -> KushnResult<R>,
    {
        let _lock = lock_env_guard();
        let _guard = WorkingDirGuard::set(dir)?;
        func()
    }

    #[test]
    fn calculate_file_hash_returns_expected_digest() -> KushnResult<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("hello.txt");
        fs::write(&file_path, b"hello world")?;

        let hash = calculate_file_hash(&file_path)?;
        let expected = format!("{:x}", Sha256::digest(b"hello world"));

        assert_eq!(hash, expected);
        Ok(())
    }

    #[test]
    fn process_file_respects_ignore_patterns() -> KushnResult<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("ignored.txt");
        fs::write(&file_path, b"ignore me")?;

        with_working_dir(dir.path(), || {
            let result = process_file("ignored.txt", &[String::from("ignored.txt")])?;
            assert!(result.is_none());
            Ok(())
        })
    }

    #[test]
    fn process_file_returns_hash_for_included_file() -> KushnResult<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("include.txt");
        fs::write(&file_path, b"include me")?;

        with_working_dir(dir.path(), || {
            let result = process_file("include.txt", &[])?;
            let file_hash = result.expect("expected file hash entry");
            assert_eq!(file_hash.path, "include.txt");
            let expected = format!("{:x}", Sha256::digest(b"include me"));
            assert_eq!(file_hash.hash, expected);
            Ok(())
        })
    }

    #[test]
    fn process_directory_skips_ignored_entries() -> KushnResult<()> {
        let dir = tempdir()?;
        let keep_file = dir.path().join("keep.txt");
        fs::write(&keep_file, b"keep")?;

        let skip_dir = dir.path().join("skip");
        fs::create_dir(&skip_dir)?;
        fs::write(skip_dir.join("ignored.txt"), b"ignored")?;

        with_working_dir(dir.path(), || {
            let current_dir = env::current_dir()?;
            let hashes = process_directory(&current_dir, &[String::from("skip")])?;
            let mut paths: Vec<_> = hashes.into_iter().map(|entry| entry.path).collect();
            paths.sort();
            assert_eq!(paths, vec![String::from("keep.txt")]);
            Ok(())
        })
    }
}

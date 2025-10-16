use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::{env, fs, io};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Serialize, Deserialize)]
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

pub fn calculate_file_hash<P: AsRef<Path>>(file_path: P) -> KushnResult<String> {
    let mut file = fs::File::open(file_path)?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)?;
    let hash_result = hasher.finalize();
    Ok(format!("{:x}", hash_result))
}

pub fn process_file<P: AsRef<Path>>(
    file_path: P,
    ignore: &[String],
) -> KushnResult<Option<FileHash>> {
    let file_path = file_path.as_ref();
    let base_dir = env::current_dir()?;
    let relative_path = file_path.strip_prefix(&base_dir).unwrap_or(file_path);

    let ignore_patterns = ignore
        .iter()
        .map(|pattern| {
            let normalized = pattern.replace('\\', "/");
            glob::Pattern::new(&format!("**/{}", normalized))
        })
        .collect::<Result<Vec<_>, _>>()?;

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
            if let Some(relative_path_str) = relative_path.to_str() {
                let normalized_relative = relative_path_str.replace('\\', "/");
                if directory_ignore_patterns
                    .iter()
                    .any(|pattern| pattern.matches(&normalized_relative))
                {
                    continue;
                }
            }

            if let Some(file_hash) = process_file(path, ignore)? {
                results.push(file_hash);
            }
        }
    }

    Ok(results)
}

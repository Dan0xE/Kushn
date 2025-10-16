use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::{env, fs, io};
use walkdir::WalkDir;

#[derive(Serialize, Deserialize)]
pub struct FileHash {
    pub path: String,
    pub hash: String,
}

pub fn calculate_file_hash<P: AsRef<Path>>(file_path: P) -> Result<String, io::Error> {
    let mut file = fs::File::open(file_path)?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)?;
    let hash_result = hasher.finalize();
    Ok(format!("{:x}", hash_result))
}

pub fn process_file<P: AsRef<Path>>(
    file_path: P,
    ignore: &[String],
) -> Result<Option<FileHash>, io::Error> {
    let file_path = file_path.as_ref();
    let relative_path = file_path
        .strip_prefix(env::current_dir()?)
        .map_err(io::Error::other)?;

    let ignore_patterns = ignore
        .iter()
        .map(|pattern| glob::Pattern::new(&format!("**/{}", pattern)))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    if ignore_patterns
        .iter()
        .any(|pattern| pattern.matches_path_with(relative_path, glob::MatchOptions::new()))
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

pub fn process_directory<P: AsRef<Path>>(directory_path: P, ignore: &[String]) -> Vec<FileHash> {
    let mut results = Vec::new();

    for entry in WalkDir::new(&directory_path).follow_links(true).into_iter() {
        if let Ok(entry) = entry {
            let path = entry.path();
            let relative_path = path.strip_prefix(&directory_path).unwrap_or(path);

            if entry.file_type().is_dir() {
                if ignore.iter().any(|pattern| {
                    let full_pattern = format!("{}/**", pattern.replace("\\", "/"));
                    let pattern = glob::Pattern::new(&full_pattern).unwrap();
                    pattern.matches_path_with(relative_path, glob::MatchOptions::new())
                }) {
                    continue;
                }
            } else if let Some(relative_path_str) = relative_path.to_str()
                && ignore.iter().any(|pattern| {
                    let full_pattern = format!("{}/**", pattern.replace("\\", "/"));
                    let pattern = glob::Pattern::new(&full_pattern).unwrap();
                    pattern.matches(relative_path_str)
                })
            {
                continue;
            }

            if let Ok(Some(file_hash)) = process_file(path, ignore) {
                results.push(file_hash);
            }
        } else if let Err(err) = entry {
            eprintln!("Error processing entry: {:?}", err);
        }
    }

    results
}

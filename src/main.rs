use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

#[derive(Serialize, Deserialize)]
struct FileHash {
    path: String,
    hash: String,
}

fn calculate_file_hash<P: AsRef<Path>>(file_path: P) -> Result<String, io::Error> {
    let mut file = fs::File::open(file_path)?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)?;
    let hash_result = hasher.finalize();
    Ok(format!("{:x}", hash_result))
}

fn process_file<P: AsRef<Path>>(
    file_path: P,
    ignore: &Vec<String>,
) -> Result<Option<FileHash>, io::Error> {
    let file_path = file_path.as_ref();
    let relative_path = file_path
        .strip_prefix(env::current_dir()?)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let ignore_patterns = ignore
        .iter()
        .map(|pattern| glob::Pattern::new(&format!("**/{}", pattern)))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    if ignore_patterns
        .iter()
        .any(|pattern| pattern.matches_path_with(&relative_path, glob::MatchOptions::new()))
    {
        return Ok(None);
    }

    let hash = calculate_file_hash(&file_path)?;
    let path_string = relative_path.to_string_lossy().into_owned();
    Ok(Some(FileHash {
        path: path_string,
        hash,
    }))
}

fn process_directory<P: AsRef<Path>>(directory_path: P, ignore: &Vec<String>) -> Vec<FileHash> {
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
                    continue; // Skip the entire directory
                }
            } else {
                if let Some(relative_path_str) = relative_path.to_str() {
                    if ignore.iter().any(|pattern| {
                        let full_pattern = format!("{}/**", pattern.replace("\\", "/"));
                        let pattern = glob::Pattern::new(&full_pattern).unwrap();
                        pattern.matches(relative_path_str)
                    }) {
                        continue; // Skip files within ignored directory
                    }
                }
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

fn main() {
    let current_dir = env::current_dir().expect("Failed to get current directory.");
    let ignore_file_path = current_dir.join(".kushnignore");

    let ignore_patterns: Vec<String> = if ignore_file_path.exists() {
        let ignore_contents =
            fs::read_to_string(ignore_file_path).expect("Failed to read .kushnignore file.");
        ignore_contents
            .lines()
            .map(|s| s.trim().to_string())
            .collect()
    } else {
        Vec::new()
    };

    let file_hashes = process_directory(&current_dir, &ignore_patterns);

    let output_file_name = match env::args().position(|arg| arg == "--name") {
        Some(index) => {
            let output_file_arg = env::args().nth(index + 1);
            match output_file_arg {
                Some(filename) => filename,
                None => {
                    eprintln!("No filename provided after --name flag. Using default name kushn_result.json.");
                    "kushn_result.json".to_owned()
                }
            }
        }
        None => "kushn_result.json".to_owned(),
    };

    let output_file_path = current_dir.join(output_file_name.clone());
    let output_file = fs::File::create(output_file_path).expect("Failed to create output file.");

    let json_output =
        serde_json::to_string_pretty(&file_hashes).expect("Failed to convert file hashes to JSON.");

    io::BufWriter::new(output_file)
        .write_all(json_output.as_bytes())
        .expect("Failed to write JSON output to file.");

    println!("File hashes generated and saved to {}.", output_file_name);
}

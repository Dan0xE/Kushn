use clap::Parser;
use std::env;
use std::fs;
use std::io;
use std::path::Path;

use kushn::{FileHash, KushnError, calculate_file_hash, process_directory};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Output file name for the generated hashes manifest
    #[arg(short, long, value_name = "FILE", default_value = "kushn_result.json")]
    name: String,
}

fn main() -> Result<(), KushnError> {
    let cli = Cli::parse();
    let current_dir = env::current_dir()?;
    let ignore_file_path = current_dir.join(".kushnignore");

    let ignore_patterns: Vec<String> = if ignore_file_path.exists() {
        fs::read_to_string(ignore_file_path)?
            .lines()
            .map(|s| s.trim().to_string())
            .collect()
    } else {
        Vec::new()
    };

    let mut file_hashes = process_directory(&current_dir, &ignore_patterns)?;
    let output_file_name = cli.name;

    let output_file_path = current_dir.join(&output_file_name);
    write_hashes_to_path(&output_file_path, &file_hashes)?;

    let result_file_hash = calculate_file_hash(&output_file_path)?;
    let result_file_entry = FileHash {
        path: output_file_name.clone(),
        hash: result_file_hash,
    };
    file_hashes.push(result_file_entry);

    // NOTE reason to why we write the file twice is to ensure
    // that the output file itself is included in the hash manifest
    write_hashes_to_path(&output_file_path, &file_hashes)?;

    println!("File hashes generated and saved to {}.", output_file_name);
    Ok(())
}

fn write_hashes_to_path(path: &Path, file_hashes: &[FileHash]) -> Result<(), KushnError> {
    let file = fs::File::create(path)?;
    let writer = io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, file_hashes)?;
    Ok(())
}

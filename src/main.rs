use clap::Parser;
use std::env;
use std::fs;
use std::io::{self, Write};

use kushn::{FileHash, calculate_file_hash, process_directory};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Output file name for the generated hashes manifest
    #[arg(short, long, value_name = "FILE", default_value = "kushn_result.json")]
    name: String,
}

fn main() {
    let cli = Cli::parse();
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

    let mut file_hashes = process_directory(&current_dir, &ignore_patterns);
    let output_file_name = cli.name;

    let output_file_path = current_dir.join(&output_file_name);
    let output_file = fs::File::create(&output_file_path).expect("Failed to create output file.");

    let json_output =
        serde_json::to_string_pretty(&file_hashes).expect("Failed to convert file hashes to JSON.");

    io::BufWriter::new(&output_file)
        .write_all(json_output.as_bytes())
        .expect("Failed to write JSON output to file.");

    let result_file_hash =
        calculate_file_hash(&output_file_path).expect("Failed to calculate file hash.");
    let result_file_entry = FileHash {
        path: output_file_name.clone(),
        hash: result_file_hash,
    };
    file_hashes.push(result_file_entry);

    let output_file = fs::File::create(&output_file_path).expect("Failed to create output file.");
    let json_output =
        serde_json::to_string_pretty(&file_hashes).expect("Failed to convert file hashes to JSON.");
    io::BufWriter::new(output_file)
        .write_all(json_output.as_bytes())
        .expect("Failed to write JSON output to file.");

    println!("File hashes generated and saved to {}.", output_file_name);
}

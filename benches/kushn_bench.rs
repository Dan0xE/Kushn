use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use kushn::{calculate_file_hash, process_directory};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

struct WorkingDirGuard {
    original: PathBuf,
}

impl WorkingDirGuard {
    fn change_to(path: &Path) -> Self {
        let original = env::current_dir().expect("failed to capture current dir");
        env::set_current_dir(path).expect("failed to switch working directory");
        Self { original }
    }
}

impl Drop for WorkingDirGuard {
    fn drop(&mut self) {
        let _ = env::set_current_dir(&self.original);
    }
}

fn bench_calculate_file_hash(c: &mut Criterion) {
    let dir = tempdir().expect("failed to create temp directory");
    let file_path = dir.path().join("sample.bin");
    fs::write(&file_path, vec![0u8; 64 * 1024]).expect("failed to seed sample file");

    c.bench_function("calculate_file_hash 64KB", |b| {
        b.iter(|| {
            let hash = calculate_file_hash(black_box(&file_path)).expect("hashing failed");
            black_box(hash);
        });
    });
}

fn bench_process_directory(c: &mut Criterion) {
    c.bench_function("process_directory 100 files", |b| {
        b.iter_batched(
            || {
                let dir = tempdir().expect("failed to create temp directory");
                for i in 0..100 {
                    let path = dir.path().join(format!("file_{i:03}.txt"));
                    fs::write(path, b"benchmark data").expect("failed to seed benchmark file");
                }
                dir
            },
            |dir| {
                let guard = WorkingDirGuard::change_to(dir.path());
                let hashes =
                    process_directory(Path::new("."), &[]).expect("directory processing failed");
                black_box(hashes.len());
                drop(guard);
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, bench_calculate_file_hash, bench_process_directory);
criterion_main!(benches);

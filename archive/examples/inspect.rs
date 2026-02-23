use std::path::Path;

use archive::{AssetPool, FileArchive};

fn inspect(path: &Path) {
    println!("=== {} ===", path.display());

    let archive = match FileArchive::open(path) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("  ERROR: {e}");
            return;
        }
    };

    let entries = archive.entries();
    println!("  entries: {}", archive.len());

    let show_first = 10.min(entries.len());
    for entry in &entries[..show_first] {
        println!(
            "  [{:>8}] {:>8} bytes  {}",
            entry.offset, entry.size, entry.name
        );
    }

    if entries.len() > 15 {
        println!("  ...");
        for entry in &entries[entries.len() - 5..] {
            println!(
                "  [{:>8}] {:>8} bytes  {}",
                entry.offset, entry.size, entry.name
            );
        }
    }

    // Validate all entries are within bounds
    let file_len = path.metadata().map(|m| m.len()).unwrap_or(0);
    let mut issues = 0;
    for entry in entries {
        let end = entry.offset as u64 + entry.size as u64;
        if end > file_len {
            if issues < 3 {
                eprintln!(
                    "  WARN: {} offset {}+{} exceeds file size {}",
                    entry.name, entry.offset, entry.size, file_len
                );
            }
            issues += 1;
        }
    }
    if issues > 3 {
        eprintln!("  ... and {} more issues", issues - 3);
    }
    if issues == 0 {
        println!("  OK: all entries within file bounds");
    }
    println!();
}

fn main() {
    let assets = Path::new("assets");
    if !assets.is_dir() {
        eprintln!("Run from workspace root (assets/ directory not found)");
        std::process::exit(1);
    }

    for name in ["khan.dat", "legend.dat", "seo.dat"] {
        let path = assets.join(name);
        if path.exists() {
            inspect(&path);
        } else {
            eprintln!("{} not found, skipping", path.display());
        }
    }

    // Test the asset pool
    println!("=== AssetPool ===");
    match AssetPool::load(assets) {
        Ok(pool) => {
            println!(
                "  {} archives, {} unique assets",
                pool.archive_count(),
                pool.len()
            );

            // Spot-check a few known files
            for name in ["ma00101.epf", "0.mp3", "SOTP.DAT"] {
                match pool.get(name) {
                    Some(data) => println!("  {name}: {} bytes", data.len()),
                    None => println!("  {name}: NOT FOUND"),
                }
            }
        }
        Err(e) => eprintln!("  ERROR: {e}"),
    }
}

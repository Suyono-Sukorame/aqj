use anyhow::{Context, Result};
use aqj_core::LocalDb;
use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "aqj-remove", author, version, about = "Remove installed AQJ packages")]
struct Cli {
    /// Target root directory (default: /)
    #[arg(short, long, default_value = "/")]
    root: PathBuf,

    /// Name of installed package to remove
    package_name: String,
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    remove_package(&cli.package_name, &cli.root)?;
    Ok(())
}

pub fn remove_package(package_name: &str, root: &Path) -> Result<()> {
    let mut db = LocalDb::new(root)
        .with_context(|| format!("Failed to load AQJ database at {}", root.display()))?;

    if !db.is_installed(package_name) {
        println!("--> Package '{}' is not installed.", package_name);
        return Ok(());
    }

    println!("--> Removing package '{}' from {}...", package_name, root.display());

    let installed_pkg = db.unregister_package(package_name)
        .with_context(|| format!("Failed to unregister package '{}'", package_name))?;

    let mut removed_count = 0;

    for file in &installed_pkg.files {
        let full_path = root.join(&file.path);
        if full_path.exists() {
            if full_path.is_file() {
                if let Err(e) = fs::remove_file(&full_path) {
                    eprintln!("Warning: Failed to remove file {}: {}", full_path.display(), e);
                } else {
                    removed_count += 1;
                }
            }

            // Clean up parent directory if empty
            if let Some(parent) = full_path.parent() {
                let _ = fs::remove_dir(parent); // Fails silently if directory not empty
            }
        }
    }

    println!("--> [SUCCESS] Package '{}' removed! (Removed {} files)", package_name, removed_count);
    Ok(())
}

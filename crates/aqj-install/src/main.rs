use anyhow::{anyhow, Context, Result};
use aqj_core::{InstalledPackage, LocalDb, PackageArchive};
use clap::Parser;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Parser)]
#[command(name = "aqj-install", author, version, about = "Install AQJ binary packages")]
struct Cli {
    /// Target root directory (default: /)
    #[arg(short, long, default_value = "/")]
    root: PathBuf,

    /// Path to .aqj package file
    package_file: PathBuf,
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    install_package(&cli.package_file, &cli.root)?;
    Ok(())
}

pub fn install_package(package_file: &Path, root: &Path) -> Result<()> {
    if !package_file.exists() {
        return Err(anyhow!("Package file not found: {}", package_file.display()));
    }

    println!("--> Inspecting package archive: {}", package_file.display());
    let (metadata, _) = PackageArchive::inspect(package_file)
        .with_context(|| "Failed to read package archive metadata")?;

    let mut db = LocalDb::new(root)
        .with_context(|| format!("Failed to load AQJ database at {}", root.display()))?;

    if db.is_installed(&metadata.name) {
        println!("--> Package '{}' is already installed. Reinstalling...", metadata.name);
    }

    println!("--> Installing {} ({}) to {}...", metadata.name, metadata.full_version(), root.display());

    let (_, pkg_files) = PackageArchive::extract(package_file, root)
        .with_context(|| "Failed to extract package files to target root")?;

    let install_date = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let installed_pkg = InstalledPackage {
        metadata: metadata.clone(),
        install_date,
        files: pkg_files,
    };

    db.register_package(installed_pkg)
        .with_context(|| "Failed to register package in LocalDb")?;

    println!("--> [SUCCESS] Package '{}' installed successfully!", metadata.name);
    Ok(())
}

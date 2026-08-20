use anyhow::{anyhow, Context, Result};
use aqj_core::{AqjConfig, InstalledPackage, LocalDb, PackageArchive, PackageDownloader, RepoCache};
use clap::Parser;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Parser)]
#[command(name = "aqj-install", author, version, about = "Install AQJ binary packages (from file or repository)")]
struct Cli {
    /// Target root directory (default: /)
    #[arg(short, long, default_value = "/")]
    root: PathBuf,

    /// AQJ configuration file (used for remote repo lookup)
    #[arg(short, long, default_value = "/etc/aqj/repos.conf")]
    config: PathBuf,

    /// Path to .aqj file OR package name to fetch from a remote repository
    package: String,
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    // If the argument looks like a local file path, install it directly.
    // Otherwise treat it as a package name and fetch from repo.
    let pkg_path = PathBuf::from(&cli.package);
    if pkg_path.exists() || cli.package.ends_with(".aqj") {
        install_package(&pkg_path, &cli.root)?;
    } else {
        install_from_repo(&cli.package, &cli.root, &cli.config)?;
    }
    Ok(())
}

/// Install a local `.aqj` archive to `root`.
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

/// Resolve a package name from cached repository indexes, download it, and install.
pub fn install_from_repo(pkg_name: &str, root: &Path, config_path: &Path) -> Result<()> {
    let config = AqjConfig::load_from(config_path).unwrap_or_else(|e| {
        eprintln!("[WARN] Could not read config: {}. Using defaults.", e);
        AqjConfig::default()
    });

    let cache = RepoCache::new(config.cache_index_dir());
    let downloader = PackageDownloader::new(config.cache_pkg_dir());

    println!("--> Looking up '{}' in repository indexes...", pkg_name);
    let (_, repo_pkg) = cache
        .find_package(pkg_name)
        .ok_or_else(|| anyhow!(
            "Package '{}' not found in any cached repository index.\n\
             Run 'aqj sync update' to refresh repository indexes.",
            pkg_name
        ))?;

    println!(
        "--> Found: {} {}-{}_{} — {}",
        repo_pkg.name, repo_pkg.version, repo_pkg.revision, repo_pkg.architecture, repo_pkg.summary
    );

    let pkg_file = downloader.download(&repo_pkg)
        .with_context(|| format!("Failed to download package '{}'", pkg_name))?;

    install_package(&pkg_file, root)
}

use anyhow::{anyhow, Context, Result};
use aqj_core::{calculate_sha256, PackageArchive, PackageMetadata};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Command;
use tar::Archive as TarArchive;
use flate2::read::GzDecoder;

#[derive(Parser)]
#[command(name = "aqj-src", author, version, about = "AQJ Package Builder from Source Recipe")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build a package from a recipe template
    Build {
        /// Package name or path to recipe template.toml
        target: String,

        /// Output directory for resulting .aqj file
        #[arg(short, long, default_value = "build/binaries")]
        output_dir: PathBuf,

        /// Work/build directory
        #[arg(short, long, default_value = "build/work")]
        work_dir: PathBuf,
    },
}

#[derive(Debug, Deserialize)]
struct Recipe {
    package: PackageMetadata,
    source: Option<RecipeSource>,
    build: RecipeBuild,
}

#[derive(Debug, Deserialize)]
struct RecipeSource {
    url: String,
    sha256: String,
}

#[derive(Debug, Deserialize)]
struct RecipeBuild {
    script: String,
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Build { target, output_dir, work_dir } => {
            run_build(&target, &output_dir, &work_dir)?;
        }
    }

    Ok(())
}

fn run_build(target: &str, output_dir: &Path, work_dir: &Path) -> Result<()> {
    let template_path = resolve_template_path(target)?;
    println!("--> Reading recipe template: {}", template_path.display());

    let content = fs::read_to_string(&template_path)
        .with_context(|| format!("Failed to read recipe file at {}", template_path.display()))?;

    let recipe: Recipe = toml::from_str(&content)
        .with_context(|| "Failed to parse template.toml recipe")?;

    println!("--> Building package: {} ({})", recipe.package.name, recipe.package.full_version());

    let pkg_work_dir = work_dir.join(&recipe.package.name);
    let distfiles_dir = work_dir.join("distfiles");
    let src_dir = pkg_work_dir.join("src");
    let dest_dir = pkg_work_dir.join("destdir");

    fs::create_dir_all(&distfiles_dir)?;
    fs::create_dir_all(&src_dir)?;
    fs::create_dir_all(&dest_dir)?;

    // Step 1: Download & Verify Source (if source URL specified)
    if let Some(ref src_info) = recipe.source {
        let filename = src_info.url.split('/').last().unwrap_or("source.tar.gz");
        let downloaded_file = distfiles_dir.join(filename);

        if !downloaded_file.exists() {
            println!("--> Downloading source: {}", src_info.url);
            let response = ureq::get(&src_info.url).call()
                .map_err(|e| anyhow!("Failed to download source: {}", e))?;
            let mut reader = response.into_reader();
            let mut file = File::create(&downloaded_file)?;
            std::io::copy(&mut reader, &mut file)?;
        }

        println!("--> Verifying SHA256 checksum...");
        let actual_hash = calculate_sha256(&downloaded_file)?;
        if actual_hash != src_info.sha256 {
            return Err(anyhow!(
                "Checksum mismatch for source file!\nExpected: {}\nActual:   {}",
                src_info.sha256, actual_hash
            ));
        }
        println!("--> Checksum verified: {}", actual_hash);

        // Extract tar.gz source
        println!("--> Extracting source to work directory...");
        extract_tar_gz(&downloaded_file, &src_dir)?;
    }

    // Step 2: Run Build Script inside src_dir with DESTDIR
    println!("--> Executing build script...");
    let absolute_dest_dir = fs::canonicalize(&dest_dir)
        .unwrap_or_else(|_| dest_dir.clone());

    let status = Command::new("bash")
        .arg("-c")
        .arg(&recipe.build.script)
        .current_dir(&src_dir)
        .env("DESTDIR", &absolute_dest_dir)
        .env("PKGNAME", &recipe.package.name)
        .env("PKGVER", &recipe.package.version)
        .status()
        .with_context(|| "Failed to execute build script with bash")?;

    if !status.success() {
        return Err(anyhow!("Build script failed with status: {}", status));
    }

    // Step 3: Package staging destdir into .aqj file
    let output_file = output_dir.join(recipe.package.archive_filename());
    println!("--> Packaging into {}", output_file.display());

    PackageArchive::create(&recipe.package, &absolute_dest_dir, &output_file)?;

    println!("--> [SUCCESS] Package created: {}", output_file.display());
    Ok(())
}

fn resolve_template_path(target: &str) -> Result<PathBuf> {
    let target_path = PathBuf::from(target);
    if target_path.exists() {
        if target_path.is_file() {
            return Ok(target_path);
        } else if target_path.join("template.toml").exists() {
            return Ok(target_path.join("template.toml"));
        }
    }

    // Check aqj-packages/pkgs/<target>/template.toml
    let pkg_repo_path = PathBuf::from("aqj-packages/pkgs").join(target).join("template.toml");
    if pkg_repo_path.exists() {
        return Ok(pkg_repo_path);
    }

    Err(anyhow!("Could not find recipe template for target: {}", target))
}

fn extract_tar_gz(archive: &Path, target_dir: &Path) -> Result<()> {
    let file = File::open(archive)?;
    let gz = GzDecoder::new(BufReader::new(file));
    let mut tar = TarArchive::new(gz);

    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        let mut components = path.components();
        
        // Strip top-level directory if present
        components.next();
        let rel_path: PathBuf = components.collect();

        if rel_path.as_os_str().is_empty() {
            continue;
        }

        let dest = target_dir.join(rel_path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        entry.unpack(&dest)?;
    }

    Ok(())
}

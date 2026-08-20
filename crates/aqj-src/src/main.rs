use anyhow::{anyhow, Context, Result};
use aqj_core::{calculate_sha256, DependencySolver, PackageArchive, PackageMetadata};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::collections::HashMap;
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

        /// Local recipe repository directory (scanned for dependency resolution)
        #[arg(short, long, default_value = "aqj-packages/pkgs")]
        repo_dir: PathBuf,
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
        Commands::Build { target, output_dir, work_dir, repo_dir } => {
            run_build(&target, &output_dir, &work_dir, &repo_dir)?;
        }
    }

    Ok(())
}

/// Scan a recipe repository directory and load all available package metadata.
fn scan_recipe_repo(repo_dir: &Path) -> Result<(HashMap<String, PackageMetadata>, HashMap<String, Recipe>)> {
    let mut available: HashMap<String, PackageMetadata> = HashMap::new();
    let mut recipes: HashMap<String, Recipe> = HashMap::new();

    if !repo_dir.exists() {
        return Ok((available, recipes));
    }

    for entry in fs::read_dir(repo_dir)? {
        let entry = entry?;
        let template_path = entry.path().join("template.toml");
        if !template_path.is_file() {
            continue;
        }

        let content = match fs::read_to_string(&template_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let recipe: Recipe = match toml::from_str(&content) {
            Ok(r) => r,
            Err(_) => continue,
        };

        let name = recipe.package.name.clone();
        available.insert(name.clone(), recipe.package.clone());
        recipes.insert(name, recipe);
    }

    Ok((available, recipes))
}

fn run_build(target: &str, output_dir: &Path, work_dir: &Path, repo_dir: &Path) -> Result<()> {
    let template_path = resolve_template_path(target, repo_dir)?;
    println!("--> Reading recipe template: {}", template_path.display());

    let content = fs::read_to_string(&template_path)
        .with_context(|| format!("Failed to read recipe file at {}", template_path.display()))?;
    let target_recipe: Recipe = toml::from_str(&content)
        .with_context(|| "Failed to parse template.toml recipe")?;

    let target_name = target_recipe.package.name.clone();

    // ─── Dependency Resolution ─────────────────────────────────────────────────
    println!("--> Scanning local recipe repository at '{}' for dependency resolution...", repo_dir.display());
    let (mut available, recipes) = scan_recipe_repo(repo_dir)?;

    // Ensure the target itself is in the universe (it may be outside the repo dir)
    available
        .entry(target_name.clone())
        .or_insert_with(|| target_recipe.package.clone());

    if target_recipe.package.depends.is_empty() {
        println!("--> No dependencies declared for '{}'.", target_name);
    } else {
        println!(
            "--> Resolving dependencies for '{}': {:?}",
            target_name, target_recipe.package.depends
        );
        let build_order = DependencySolver::resolve(&[target_name.as_str()], &available)
            .map_err(|e| anyhow!("Dependency resolution failed: {}", e))?;

        let dep_order: Vec<&str> = build_order
            .iter()
            .map(|p| p.name.as_str())
            .filter(|&n| n != target_name.as_str())
            .collect();

        if dep_order.is_empty() {
            println!("--> All dependencies already satisfied.");
        } else {
            println!("--> Build order (dependencies first): {:?}", dep_order);
            for dep_name in &dep_order {
                if let Some(dep_recipe) = recipes.get(*dep_name) {
                    if output_dir.join(dep_recipe.package.archive_filename()).exists() {
                        println!("--> [SKIP] Dependency '{}' archive already exists.", dep_name);
                        continue;
                    }
                    println!("==> Building dependency: {}", dep_name);
                    build_recipe(dep_recipe, output_dir, work_dir)?;
                } else {
                    println!("--> [WARN] Dependency '{}' recipe not found locally — assuming pre-installed.", dep_name);
                }
            }
        }
    }
    // ──────────────────────────────────────────────────────────────────────────

    println!("==> Building target package: {} ({})", target_recipe.package.name, target_recipe.package.full_version());
    build_recipe(&target_recipe, output_dir, work_dir)?;

    Ok(())
}

/// Core build logic: download, verify, compile, and package a single recipe.
fn build_recipe(recipe: &Recipe, output_dir: &Path, work_dir: &Path) -> Result<()> {
    let pkg_work_dir = work_dir.join(&recipe.package.name);
    let distfiles_dir = work_dir.join("distfiles");
    let src_dir = pkg_work_dir.join("src");
    let dest_dir = pkg_work_dir.join("destdir");

    fs::create_dir_all(&distfiles_dir)?;
    fs::create_dir_all(&src_dir)?;
    fs::create_dir_all(&dest_dir)?;

    // Step 1: Download & Verify Source
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
        } else {
            println!("--> Source already cached: {}", downloaded_file.display());
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

        println!("--> Extracting source to work directory...");
        extract_tar_gz(&downloaded_file, &src_dir)?;
    }

    // Step 2: Run Build Script
    println!("--> Executing build script...");
    let absolute_dest_dir = fs::canonicalize(&dest_dir).unwrap_or_else(|_| dest_dir.clone());

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

    // Step 3: Package into .aqj archive
    fs::create_dir_all(output_dir)?;
    let output_file = output_dir.join(recipe.package.archive_filename());
    println!("--> Packaging into {}", output_file.display());

    PackageArchive::create(&recipe.package, &absolute_dest_dir, &output_file)?;
    println!("--> [SUCCESS] Package created: {}", output_file.display());

    Ok(())
}

fn resolve_template_path(target: &str, repo_dir: &Path) -> Result<PathBuf> {
    let target_path = PathBuf::from(target);
    if target_path.exists() {
        if target_path.is_file() {
            return Ok(target_path);
        } else if target_path.join("template.toml").exists() {
            return Ok(target_path.join("template.toml"));
        }
    }

    // Check <repo_dir>/<target>/template.toml
    let pkg_repo_path = repo_dir.join(target).join("template.toml");
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

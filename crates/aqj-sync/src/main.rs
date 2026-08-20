use anyhow::{anyhow, Result};
use aqj_core::{AqjConfig, RepoCache};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "aqj-sync",
    author,
    version,
    about = "AQJ Repository Sync — Manage and sync remote package repository indexes"
)]
struct Cli {
    /// AQJ configuration file path
    #[arg(short, long, default_value = "/etc/aqj/repos.conf")]
    config: PathBuf,

    /// Override repository cache directory
    #[arg(long)]
    cache_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Sync (download/refresh) index from all configured repositories
    Update,

    /// List all configured repositories and their sync status
    List,

    /// Search for packages matching a query across all cached indexes
    Search {
        /// Search query (matches package name or summary)
        query: String,
    },

    /// Show detailed information about a specific package from the remote index
    Info {
        /// Package name to look up
        package: String,
    },
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    let config = AqjConfig::load_from(&cli.config).unwrap_or_else(|e| {
        eprintln!("[WARN] Could not read config ({}), using defaults.", e);
        AqjConfig::default()
    });

    let cache_dir = cli
        .cache_dir
        .unwrap_or_else(|| config.cache_index_dir());
    let cache = RepoCache::new(&cache_dir);

    match cli.command {
        Commands::Update => cmd_update(&config, &cache),
        Commands::List => cmd_list(&config, &cache),
        Commands::Search { query } => cmd_search(&cache, &query),
        Commands::Info { package } => cmd_info(&cache, &package),
    }
}

/// Download and refresh all enabled repository indexes.
fn cmd_update(config: &AqjConfig, cache: &RepoCache) -> Result<()> {
    let repos = config.enabled_repos();
    if repos.is_empty() {
        println!("No repositories configured. Add entries to /etc/aqj/repos.conf.");
        return Ok(());
    }

    let mut ok = 0;
    let mut fail = 0;
    for repo in &repos {
        match cache.sync(&repo.name, &repo.url) {
            Ok(index) => {
                println!(
                    "[{}] Synced: {} packages available.",
                    repo.name,
                    index.packages.len()
                );
                ok += 1;
            }
            Err(e) => {
                eprintln!("[{}] FAILED: {}", repo.name, e);
                fail += 1;
            }
        }
    }

    println!("\nSync complete: {}/{} repositories updated.", ok, ok + fail);
    if fail > 0 {
        return Err(anyhow!("{} repository/repositories failed to sync.", fail));
    }
    Ok(())
}

/// List configured repos with their last-sync status.
fn cmd_list(config: &AqjConfig, cache: &RepoCache) -> Result<()> {
    let repos = &config.repos;
    if repos.is_empty() {
        println!("No repositories configured.");
        return Ok(());
    }

    println!("{:<20} {:<10} {:<10} {}", "NAME", "ENABLED", "PACKAGES", "URL");
    println!("{}", "-".repeat(70));
    for repo in repos {
        let pkg_count = cache
            .load(&repo.name)
            .map(|idx| idx.packages.len().to_string())
            .unwrap_or_else(|_| "not synced".to_string());

        let enabled_str = if repo.enabled { "yes" } else { "no" };
        println!("{:<20} {:<10} {:<10} {}", repo.name, enabled_str, pkg_count, repo.url);
    }
    Ok(())
}

/// Search across all cached indexes.
fn cmd_search(cache: &RepoCache, query: &str) -> Result<()> {
    let results = cache.search_all(query);
    if results.is_empty() {
        println!("No packages found matching '{}'.", query);
        return Ok(());
    }

    println!("{:<5} {:<30} {:<15} {}", "REPO", "NAME", "VERSION", "SUMMARY");
    println!("{}", "-".repeat(80));
    for (repo_name, pkg) in &results {
        println!(
            "{:<5} {:<30} {:<15} {}",
            repo_name,
            pkg.name,
            format!("{}-{}_{}", pkg.version, pkg.revision, pkg.architecture),
            pkg.summary
        );
    }
    println!("\n{} package(s) found.", results.len());
    Ok(())
}

/// Show detailed info for one package.
fn cmd_info(cache: &RepoCache, pkg_name: &str) -> Result<()> {
    let (index, pkg) = cache
        .find_package(pkg_name)
        .ok_or_else(|| anyhow!("Package '{}' not found in any cached repository index.\nRun 'aqj sync update' first.", pkg_name))?;

    println!("Package      : {}", pkg.name);
    println!("Version      : {}-{}_{}", pkg.version, pkg.revision, pkg.architecture);
    println!("Summary      : {}", pkg.summary);
    println!("License      : {}", pkg.license);
    if let Some(ref hp) = pkg.homepage {
        println!("Homepage     : {}", hp);
    }
    println!("Repository   : {}", index.name);
    println!("Download URL : {}", pkg.download_url);
    println!("SHA256       : {}", pkg.sha256);
    println!("Installed Sz : {} bytes", pkg.installed_size);
    if !pkg.depends.is_empty() {
        println!("Depends      : {}", pkg.depends.join(", "));
    } else {
        println!("Depends      : (none)");
    }
    Ok(())
}

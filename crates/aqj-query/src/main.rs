use anyhow::{Context, Result};
use aqj_core::LocalDb;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "aqj-query", author, version, about = "Query AQJ installed packages and files")]
struct Cli {
    /// Target root directory (default: /)
    #[arg(short, long, default_value = "/")]
    root: PathBuf,

    /// List all installed packages
    #[arg(short, long)]
    list: bool,

    /// Show detailed info for a package
    #[arg(short, long)]
    info: Option<String>,

    /// List files owned by a package
    #[arg(short, long)]
    files: Option<String>,

    /// Search installed packages matching query
    #[arg(short, long)]
    search: Option<String>,

    /// Find which package owns a specific file path
    #[arg(short, long)]
    owner: Option<String>,
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    run_query(&cli)?;
    Ok(())
}

fn run_query(cli: &Cli) -> Result<()> {
    let db = LocalDb::new(&cli.root)
        .with_context(|| format!("Failed to load AQJ database at {}", cli.root.display()))?;

    if cli.list {
        let pkgs = db.list_packages();
        if pkgs.is_empty() {
            println!("No installed packages found.");
        } else {
            println!("{:<20} {:<15} {:<10} {}", "PACKAGE", "VERSION", "ARCH", "SUMMARY");
            println!("{}", "-".repeat(70));
            for pkg in pkgs {
                println!(
                    "{:<20} {:<15} {:<10} {}",
                    pkg.metadata.name,
                    pkg.metadata.full_version(),
                    pkg.metadata.architecture,
                    pkg.metadata.summary
                );
            }
        }
    } else if let Some(ref pkg_name) = cli.info {
        if let Some(pkg) = db.get_package(pkg_name) {
            println!("Name         : {}", pkg.metadata.name);
            println!("Version      : {}", pkg.metadata.full_version());
            println!("Architecture : {}", pkg.metadata.architecture);
            println!("Summary      : {}", pkg.metadata.summary);
            println!("License      : {}", pkg.metadata.license);
            if let Some(ref hp) = pkg.metadata.homepage {
                println!("Homepage     : {}", hp);
            }
            println!("Dependencies : {}", pkg.metadata.depends.join(", "));
            println!("Files Count  : {}", pkg.files.len());
        } else {
            println!("Package '{}' is not installed.", pkg_name);
        }
    } else if let Some(ref pkg_name) = cli.files {
        if let Some(pkg) = db.get_package(pkg_name) {
            println!("Files owned by '{}':", pkg_name);
            for file in &pkg.files {
                println!("  /{}", file.path);
            }
        } else {
            println!("Package '{}' is not installed.", pkg_name);
        }
    } else if let Some(ref query) = cli.search {
        let q = query.to_lowercase();
        let matches: Vec<_> = db.list_packages()
            .into_iter()
            .filter(|p| p.metadata.name.to_lowercase().contains(&q) || p.metadata.summary.to_lowercase().contains(&q))
            .collect();

        if matches.is_empty() {
            println!("No packages found matching query: '{}'", query);
        } else {
            for pkg in matches {
                println!("{:<20} {:<15} - {}", pkg.metadata.name, pkg.metadata.full_version(), pkg.metadata.summary);
            }
        }
    } else if let Some(ref file_path) = cli.owner {
        let rel_path = file_path.trim_start_matches('/');
        if let Some(owner) = db.find_file_owner(rel_path) {
            println!("File '/{}' belongs to package: {}", rel_path, owner);
        } else {
            println!("No installed package owns file '/{}'", rel_path);
        }
    } else {
        println!("Use --help to view available query flags (e.g. -l, -i <pkg>, -f <pkg>, -s <query>).");
    }

    Ok(())
}

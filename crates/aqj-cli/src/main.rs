use anyhow::Result;
use clap::{Parser, Subcommand};
use std::env;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "aqj",
    author,
    version,
    about = "AQJ Linux Package Manager",
    long_about = "AQJ is a lightweight, fast package management system inspired by XBPS."
)]
struct Cli {
    /// Target root directory (default: /)
    #[arg(short, long, global = true, default_value = "/")]
    root: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install a binary package (.aqj)
    Install {
        /// Path to .aqj package file
        package_file: PathBuf,
    },
    /// Remove an installed package
    Remove {
        /// Name of package to remove
        package_name: String,
    },
    /// Query installed packages or files
    Query {
        /// List all installed packages
        #[arg(short, long)]
        list: bool,

        /// Show detailed package info
        #[arg(short, long)]
        info: Option<String>,

        /// List files owned by package
        #[arg(short, long)]
        files: Option<String>,

        /// Search packages by query
        #[arg(short, long)]
        search: Option<String>,

        /// Find package owning a file
        #[arg(short, long)]
        owner: Option<String>,
    },
    /// Build package from source recipe
    Src {
        /// Target package name or recipe path
        target: String,

        /// Output directory for resulting .aqj file
        #[arg(short, long, default_value = "build/binaries")]
        output_dir: PathBuf,

        /// Work/build directory
        #[arg(short, long, default_value = "build/work")]
        work_dir: PathBuf,
    },
}

fn main() -> Result<()> {
    env_logger::init();

    // Check multi-call invocation (e.g. if binary was called as aqj-install, aqj-remove, etc.)
    let program_name = env::args().next()
        .and_then(|p| PathBuf::from(p).file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "aqj".to_string());

    if program_name.starts_with("aqj-") {
        match program_name.as_str() {
            "aqj-install" => {
                let status = std::process::Command::new("aqj-install")
                    .args(env::args().skip(1))
                    .status()?;
                std::process::exit(status.code().unwrap_or(1));
            }
            "aqj-remove" => {
                let status = std::process::Command::new("aqj-remove")
                    .args(env::args().skip(1))
                    .status()?;
                std::process::exit(status.code().unwrap_or(1));
            }
            "aqj-query" => {
                let status = std::process::Command::new("aqj-query")
                    .args(env::args().skip(1))
                    .status()?;
                std::process::exit(status.code().unwrap_or(1));
            }
            "aqj-src" => {
                let status = std::process::Command::new("aqj-src")
                    .args(env::args().skip(1))
                    .status()?;
                std::process::exit(status.code().unwrap_or(1));
            }
            _ => {}
        }
    }

    let cli = Cli::parse();

    match cli.command {
        Commands::Install { package_file } => {
            let status = std::process::Command::new("aqj-install")
                .arg("--root")
                .arg(&cli.root)
                .arg(&package_file)
                .status()?;
            std::process::exit(status.code().unwrap_or(1));
        }
        Commands::Remove { package_name } => {
            let status = std::process::Command::new("aqj-remove")
                .arg("--root")
                .arg(&cli.root)
                .arg(&package_name)
                .status()?;
            std::process::exit(status.code().unwrap_or(1));
        }
        Commands::Query { list, info, files, search, owner } => {
            let mut cmd = std::process::Command::new("aqj-query");
            cmd.arg("--root").arg(&cli.root);
            if list { cmd.arg("--list"); }
            if let Some(i) = info { cmd.arg("--info").arg(i); }
            if let Some(f) = files { cmd.arg("--files").arg(f); }
            if let Some(s) = search { cmd.arg("--search").arg(s); }
            if let Some(o) = owner { cmd.arg("--owner").arg(o); }
            let status = cmd.status()?;
            std::process::exit(status.code().unwrap_or(1));
        }
        Commands::Src { target, output_dir, work_dir } => {
            let status = std::process::Command::new("aqj-src")
                .arg("build")
                .arg(&target)
                .arg("--output-dir")
                .arg(&output_dir)
                .arg("--work-dir")
                .arg(&work_dir)
                .status()?;
            std::process::exit(status.code().unwrap_or(1));
        }
    }
}

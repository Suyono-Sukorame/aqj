use crate::error::{AqjError, Result};
use crate::hasher::calculate_sha256;
use crate::metadata::PackageMetadata;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

/// A single package entry inside a repository index.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepoPackage {
    pub name: String,
    pub version: String,
    #[serde(default = "default_revision")]
    pub revision: u32,
    pub architecture: String,
    pub summary: String,
    pub license: String,
    pub homepage: Option<String>,
    #[serde(default)]
    pub depends: Vec<String>,
    /// SHA-256 checksum of the `.aqj` archive.
    pub sha256: String,
    /// Full download URL for the `.aqj` archive.
    pub download_url: String,
    /// Installed size in bytes (estimated).
    pub installed_size: u64,
}

fn default_revision() -> u32 {
    1
}

impl RepoPackage {
    /// Convert to a `PackageMetadata` for use in the dependency solver.
    pub fn to_package_metadata(&self) -> PackageMetadata {
        PackageMetadata {
            name: self.name.clone(),
            version: self.version.clone(),
            revision: self.revision,
            architecture: self.architecture.clone(),
            summary: self.summary.clone(),
            license: self.license.clone(),
            homepage: self.homepage.clone(),
            depends: self.depends.clone(),
            build_date: 0,
        }
    }

    /// Returns the expected archive filename for this package.
    pub fn archive_filename(&self) -> String {
        format!("{}-{}_{}.{}.aqj", self.name, self.version, self.revision, self.architecture)
    }
}

/// A full repository index (the serialized form of `index.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoIndex {
    /// Repository display name.
    pub name: String,
    /// Base URL this index was fetched from.
    pub url: String,
    /// Timestamp (UNIX seconds) of when this index was generated.
    pub generated_at: u64,
    /// All packages contained in this repository.
    pub packages: Vec<RepoPackage>,
}

impl RepoIndex {
    /// Find a package by exact name.
    pub fn find(&self, pkg_name: &str) -> Option<&RepoPackage> {
        self.packages.iter().find(|p| p.name == pkg_name)
    }

    /// Search packages whose name or summary contains `query` (case-insensitive).
    pub fn search(&self, query: &str) -> Vec<&RepoPackage> {
        let q = query.to_lowercase();
        self.packages
            .iter()
            .filter(|p| p.name.to_lowercase().contains(&q) || p.summary.to_lowercase().contains(&q))
            .collect()
    }
}

/// Manages local caches of repository indexes on disk.
pub struct RepoCache {
    /// Directory where index JSON files are stored (e.g. `/var/lib/aqj/repodata/`).
    cache_dir: PathBuf,
}

impl RepoCache {
    pub fn new<P: AsRef<Path>>(cache_dir: P) -> Self {
        Self {
            cache_dir: cache_dir.as_ref().to_path_buf(),
        }
    }

    fn index_path(&self, repo_name: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.json", repo_name))
    }

    /// Fetch the remote index from `<repo_url>/index.json` and save it locally.
    pub fn sync(&self, repo_name: &str, repo_url: &str) -> Result<RepoIndex> {
        let index_url = format!("{}/index.json", repo_url.trim_end_matches('/'));

        println!("--> Fetching index from: {}", index_url);
        let response = ureq::get(&index_url)
            .call()
            .map_err(|e| AqjError::RepoSyncFailed {
                name: repo_name.to_string(),
                url: index_url.clone(),
                message: e.to_string(),
            })?;

        let index: RepoIndex = response
            .into_json()
            .map_err(|e| AqjError::RepoSyncFailed {
                name: repo_name.to_string(),
                url: index_url.clone(),
                message: format!("Failed to parse index JSON: {}", e),
            })?;

        fs::create_dir_all(&self.cache_dir)?;
        let path = self.index_path(repo_name);
        let file = File::create(&path)?;
        serde_json::to_writer_pretty(BufWriter::new(file), &index).map_err(|e| AqjError::RepoSyncFailed {
            name: repo_name.to_string(),
            url: index_url,
            message: format!("Failed to cache index: {}", e),
        })?;

        println!(
            "--> [OK] Synced '{}': {} packages (cached to {})",
            repo_name,
            index.packages.len(),
            path.display()
        );
        Ok(index)
    }

    /// Load a locally cached index by repository name.
    pub fn load(&self, repo_name: &str) -> Result<RepoIndex> {
        let path = self.index_path(repo_name);
        if !path.exists() {
            return Err(AqjError::RepoNotFound(repo_name.to_string()));
        }
        let content = fs::read_to_string(&path)?;
        let index: RepoIndex = serde_json::from_str(&content).map_err(|e| AqjError::RepoSyncFailed {
            name: repo_name.to_string(),
            url: String::new(),
            message: format!("Corrupted local cache at {}: {}", path.display(), e),
        })?;
        Ok(index)
    }

    /// Load all cached indexes available locally.
    pub fn load_all(&self) -> Vec<RepoIndex> {
        if !self.cache_dir.exists() {
            return vec![];
        }
        fs::read_dir(&self.cache_dir)
            .map(|entries| {
                entries
                    .flatten()
                    .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
                    .filter_map(|e| {
                        let name = e.path().file_stem()?.to_string_lossy().to_string();
                        self.load(&name).ok()
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Search across all cached indexes for packages matching `query`.
    pub fn search_all(&self, query: &str) -> Vec<(String, RepoPackage)> {
        self.load_all()
            .into_iter()
            .flat_map(|idx| {
                let repo_name = idx.name.clone();
                idx.search(query)
                    .into_iter()
                    .map(|p| (repo_name.clone(), p.clone()))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Find a package by exact name across all cached indexes.
    /// Returns the first match found.
    pub fn find_package(&self, pkg_name: &str) -> Option<(RepoIndex, RepoPackage)> {
        for index in self.load_all() {
            if let Some(pkg) = index.find(pkg_name) {
                return Some((index.clone(), pkg.clone()));
            }
        }
        None
    }
}

/// Downloads a package archive from a URL, verifies its checksum, and stores it in a cache dir.
pub struct PackageDownloader {
    pub cache_dir: PathBuf,
}

impl PackageDownloader {
    pub fn new<P: AsRef<Path>>(cache_dir: P) -> Self {
        Self {
            cache_dir: cache_dir.as_ref().to_path_buf(),
        }
    }

    /// Download the package archive to the local cache. Returns the path to the downloaded file.
    /// Skips download if the file already exists and the SHA-256 matches.
    pub fn download(&self, pkg: &RepoPackage) -> Result<PathBuf> {
        fs::create_dir_all(&self.cache_dir)?;
        let dest_path = self.cache_dir.join(pkg.archive_filename());

        // Check if already cached with correct checksum
        if dest_path.exists() {
            let hash = calculate_sha256(&dest_path)?;
            if hash == pkg.sha256 {
                println!("--> [CACHE HIT] {} already cached.", pkg.archive_filename());
                return Ok(dest_path);
            }
            println!("--> Cached file checksum mismatch, re-downloading...");
        }

        println!("--> Downloading {} from {}", pkg.archive_filename(), pkg.download_url);
        let response = ureq::get(&pkg.download_url)
            .call()
            .map_err(|e| AqjError::DownloadFailed {
                url: pkg.download_url.clone(),
                message: e.to_string(),
            })?;

        let mut reader = response.into_reader();
        let mut file = File::create(&dest_path)?;
        std::io::copy(&mut reader, &mut file)?;

        // Verify checksum after download
        let actual_hash = calculate_sha256(&dest_path)?;
        if actual_hash != pkg.sha256 {
            // Remove corrupted file
            let _ = fs::remove_file(&dest_path);
            return Err(AqjError::ChecksumMismatchRemote {
                url: pkg.download_url.clone(),
                expected: pkg.sha256.clone(),
                actual: actual_hash,
            });
        }

        println!("--> [OK] Downloaded and verified: {}", dest_path.display());
        Ok(dest_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_pkg(name: &str) -> RepoPackage {
        RepoPackage {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            revision: 1,
            architecture: "x86_64".to_string(),
            summary: format!("{} package", name),
            license: "MIT".to_string(),
            homepage: None,
            depends: vec![],
            sha256: "abc123".to_string(),
            download_url: format!("https://example.com/{}.aqj", name),
            installed_size: 1024,
        }
    }

    fn make_test_index() -> RepoIndex {
        RepoIndex {
            name: "test".to_string(),
            url: "https://example.com".to_string(),
            generated_at: 0,
            packages: vec![
                make_test_pkg("hello"),
                make_test_pkg("world"),
                make_test_pkg("hello-world"),
            ],
        }
    }

    #[test]
    fn test_repo_index_find() {
        let index = make_test_index();
        assert!(index.find("hello").is_some());
        assert!(index.find("nonexistent").is_none());
    }

    #[test]
    fn test_repo_index_search() {
        let index = make_test_index();
        let results = index.search("hello");
        assert_eq!(results.len(), 2); // "hello" and "hello-world"
    }

    #[test]
    fn test_archive_filename() {
        let pkg = make_test_pkg("hello");
        assert_eq!(pkg.archive_filename(), "hello-1.0.0_1.x86_64.aqj");
    }

    #[test]
    fn test_to_package_metadata() {
        let pkg = make_test_pkg("hello");
        let meta = pkg.to_package_metadata();
        assert_eq!(meta.name, "hello");
        assert_eq!(meta.version, "1.0.0");
    }
}

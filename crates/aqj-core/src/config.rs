use crate::error::{AqjError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Default path for the AQJ configuration file.
pub const DEFAULT_CONFIG_PATH: &str = "/etc/aqj/repos.conf";

/// Represents a single configured remote repository.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepoConfig {
    /// Human-readable name for the repository (e.g. "main", "contrib").
    pub name: String,
    /// Base URL of the repository (e.g. "https://repo.example.com/aqj/x86_64").
    pub url: String,
    /// Whether this repository is active (default: true).
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Top-level AQJ configuration, read from `repos.conf`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AqjConfig {
    #[serde(rename = "repo", default)]
    pub repos: Vec<RepoConfig>,
}

impl AqjConfig {
    /// Load configuration from the default system path (`/etc/aqj/repos.conf`).
    pub fn load() -> Result<Self> {
        Self::load_from(DEFAULT_CONFIG_PATH)
    }

    /// Load configuration from an explicit path.
    pub fn load_from<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            // Return empty config if no file present yet (not an error).
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path)
            .map_err(|e| AqjError::ConfigError(format!("Failed to read {}: {}", path.display(), e)))?;
        let config: AqjConfig = toml::from_str(&content)
            .map_err(|e| AqjError::ConfigError(format!("Failed to parse {}: {}", path.display(), e)))?;
        Ok(config)
    }

    /// Write the current configuration back to the default path.
    pub fn save(&self) -> Result<()> {
        self.save_to(DEFAULT_CONFIG_PATH)
    }

    /// Write to an explicit path, creating parent directories as needed.
    pub fn save_to<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| AqjError::ConfigError(format!("Failed to serialize config: {}", e)))?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Return only enabled repositories.
    pub fn enabled_repos(&self) -> Vec<&RepoConfig> {
        self.repos.iter().filter(|r| r.enabled).collect()
    }

    /// Return directory path where repo index caches are stored.
    pub fn cache_index_dir(&self) -> PathBuf {
        PathBuf::from("/var/lib/aqj/repodata")
    }

    /// Return directory path where downloaded package archives are cached.
    pub fn cache_pkg_dir(&self) -> PathBuf {
        PathBuf::from("/var/cache/aqj")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_empty() {
        let config = AqjConfig::default();
        assert!(config.repos.is_empty());
        assert!(config.enabled_repos().is_empty());
    }

    #[test]
    fn test_parse_config_toml() {
        let toml = r#"
[[repo]]
name = "main"
url  = "https://repo.example.com/aqj/x86_64"
enabled = true

[[repo]]
name = "contrib"
url  = "https://repo.example.com/contrib/x86_64"
enabled = false
"#;
        let config: AqjConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.repos.len(), 2);
        assert_eq!(config.enabled_repos().len(), 1);
        assert_eq!(config.enabled_repos()[0].name, "main");
    }
}

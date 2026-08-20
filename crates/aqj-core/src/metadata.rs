use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageMetadata {
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
    #[serde(default)]
    pub build_date: u64,
}

fn default_revision() -> u32 {
    1
}

impl PackageMetadata {
    pub fn full_version(&self) -> String {
        format!("{}_{}", self.version, self.revision)
    }

    pub fn package_id(&self) -> String {
        format!("{}-{}_{}.{}", self.name, self.version, self.revision, self.architecture)
    }

    pub fn archive_filename(&self) -> String {
        format!("{}.aqj", self.package_id())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageFile {
    pub path: String,
    pub sha256: String,
    pub mode: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledPackage {
    pub metadata: PackageMetadata,
    pub install_date: u64,
    pub files: Vec<PackageFile>,
}

use crate::error::{AqjError, Result};
use crate::metadata::InstalledPackage;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

pub struct LocalDb {
    root_path: PathBuf,
    db_file_path: PathBuf,
    packages: HashMap<String, InstalledPackage>,
}

impl LocalDb {
    pub fn new<P: AsRef<Path>>(root: P) -> Result<Self> {
        let root_path = root.as_ref().to_path_buf();
        let db_dir = root_path.join("var/lib/aqj");
        let db_file_path = db_dir.join("installed.json");

        let mut db = Self {
            root_path,
            db_file_path,
            packages: HashMap::new(),
        };

        db.load()?;
        Ok(db)
    }

    pub fn load(&mut self) -> Result<()> {
        if !self.db_file_path.exists() {
            self.packages.clear();
            return Ok(());
        }

        let file = File::open(&self.db_file_path)?;
        let reader = BufReader::new(file);
        let packages: HashMap<String, InstalledPackage> = serde_json::from_reader(reader)
            .map_err(|e| AqjError::DatabaseError {
                path: self.db_file_path.clone(),
                message: e.to_string(),
            })?;

        self.packages = packages;
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.db_file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = File::create(&self.db_file_path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &self.packages)
            .map_err(|e| AqjError::DatabaseError {
                path: self.db_file_path.clone(),
                message: e.to_string(),
            })?;

        Ok(())
    }

    pub fn is_installed(&self, pkg_name: &str) -> bool {
        self.packages.contains_key(pkg_name)
    }

    pub fn get_package(&self, pkg_name: &str) -> Option<&InstalledPackage> {
        self.packages.get(pkg_name)
    }

    pub fn list_packages(&self) -> Vec<&InstalledPackage> {
        let mut pkgs: Vec<&InstalledPackage> = self.packages.values().collect();
        pkgs.sort_by(|a, b| a.metadata.name.cmp(&b.metadata.name));
        pkgs
    }

    pub fn find_file_owner(&self, rel_path: &str) -> Option<&str> {
        for (pkg_name, installed) in &self.packages {
            if installed.files.iter().any(|f| f.path == rel_path) {
                return Some(pkg_name.as_str());
            }
        }
        None
    }

    pub fn register_package(&mut self, installed_pkg: InstalledPackage) -> Result<()> {
        let name = installed_pkg.metadata.name.clone();

        // Check for file conflicts with other installed packages
        for file in &installed_pkg.files {
            if let Some(owner) = self.find_file_owner(&file.path) {
                if owner != name {
                    return Err(AqjError::FileConflict {
                        file: file.path.clone(),
                        owner: owner.to_string(),
                    });
                }
            }
        }

        self.packages.insert(name, installed_pkg);
        self.save()?;
        Ok(())
    }

    pub fn unregister_package(&mut self, pkg_name: &str) -> Result<InstalledPackage> {
        let installed = self.packages.remove(pkg_name)
            .ok_or_else(|| AqjError::PackageNotFound(pkg_name.to_string()))?;

        self.save()?;
        Ok(installed)
    }

    pub fn root_path(&self) -> &Path {
        &self.root_path
    }
}

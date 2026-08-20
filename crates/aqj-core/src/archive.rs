use crate::error::{AqjError, Result};
use crate::hasher::calculate_sha256;
use crate::metadata::{PackageFile, PackageMetadata};
use std::fs::{self, File};
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tar::{Archive, Builder, EntryType, Header};
use walkdir::WalkDir;
use zstd::{Decoder, Encoder};

pub struct PackageArchive;

impl PackageArchive {
    pub fn create<P: AsRef<Path>, Q: AsRef<Path>>(
        metadata: &PackageMetadata,
        staging_dir: P,
        output_file: Q,
    ) -> Result<Vec<PackageFile>> {
        let staging_path = staging_dir.as_ref();
        let output_path = output_file.as_ref();

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = File::create(output_path)?;
        let zstd_encoder = Encoder::new(file, 3)?;
        let mut builder = Builder::new(zstd_encoder);

        let mut package_files = Vec::new();

        // 1. Scan staging_dir to record package files and build archive
        for entry in WalkDir::new(staging_path) {
            let entry = entry.map_err(|e| AqjError::Archive(e.to_string()))?;
            let path = entry.path();
            
            if path == staging_path {
                continue;
            }

            let rel_path = path.strip_prefix(staging_path)
                .map_err(|e| AqjError::Archive(e.to_string()))?;
            
            let rel_path_str = rel_path.to_str()
                .ok_or_else(|| AqjError::Archive("Invalid path string".into()))?
                .to_string();

            let metadata_fs = entry.metadata()
                .map_err(|e| AqjError::Archive(e.to_string()))?;

            if entry.file_type().is_file() {
                let sha256 = calculate_sha256(path)?;
                let mode = metadata_fs.permissions().mode();

                package_files.push(PackageFile {
                    path: rel_path_str.clone(),
                    sha256,
                    mode,
                });

                let archive_file_path = Path::new("data").join(&rel_path);
                builder.append_path_with_name(path, archive_file_path)?;
            } else if entry.file_type().is_dir() {
                let archive_dir_path = Path::new("data").join(&rel_path);
                builder.append_dir_all(archive_dir_path, path)?;
            }
        }

        // 2. Serialize metadata.toml
        let metadata_toml = toml::to_string_pretty(metadata)
            .map_err(|e| AqjError::Serialization(e.to_string()))?;
        let metadata_bytes = metadata_toml.as_bytes();

        let mut header = Header::new_gnu();
        header.set_size(metadata_bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append_data(&mut header, "metadata.toml", metadata_bytes)?;

        // 3. Serialize files.json
        let files_json = serde_json::to_string_pretty(&package_files)
            .map_err(|e| AqjError::Serialization(e.to_string()))?;
        let files_bytes = files_json.as_bytes();

        let mut header = Header::new_gnu();
        header.set_size(files_bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append_data(&mut header, "files.json", files_bytes)?;

        let zstd_encoder = builder.into_inner()?;
        zstd_encoder.finish()?;

        Ok(package_files)
    }

    pub fn inspect<P: AsRef<Path>>(archive_path: P) -> Result<(PackageMetadata, Vec<PackageFile>)> {
        let file = File::open(archive_path.as_ref())?;
        let zstd_decoder = Decoder::new(file)?;
        let mut archive = Archive::new(zstd_decoder);

        let mut metadata: Option<PackageMetadata> = None;
        let mut files: Option<Vec<PackageFile>> = None;

        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.to_path_buf();

            if path == Path::new("metadata.toml") {
                let mut contents = String::new();
                entry.read_to_string(&mut contents)?;
                let meta: PackageMetadata = toml::from_str(&contents)
                    .map_err(|e| AqjError::Serialization(e.to_string()))?;
                metadata = Some(meta);
            } else if path == Path::new("files.json") {
                let mut contents = String::new();
                entry.read_to_string(&mut contents)?;
                let file_list: Vec<PackageFile> = serde_json::from_str(&contents)
                    .map_err(|e| AqjError::Serialization(e.to_string()))?;
                files = Some(file_list);
            }

            if metadata.is_some() && files.is_some() {
                break;
            }
        }

        let meta = metadata.ok_or_else(|| AqjError::MetadataMissing("metadata.toml not found in archive".into()))?;
        let files = files.unwrap_or_default();

        Ok((meta, files))
    }

    pub fn extract<P: AsRef<Path>, Q: AsRef<Path>>(
        archive_path: P,
        target_root: Q,
    ) -> Result<(PackageMetadata, Vec<PackageFile>)> {
        let target_path = target_root.as_ref();

        // First pass: Read metadata & files
        let (meta, pkg_files) = Self::inspect(archive_path.as_ref())?;

        // Second pass: Extract data/ to target_root
        let file = File::open(archive_path.as_ref())?;
        let zstd_decoder = Decoder::new(file)?;
        let mut archive = Archive::new(zstd_decoder);

        for entry in archive.entries()? {
            let mut entry = entry?;
            let entry_path = entry.path()?.to_path_buf();

            if entry_path.starts_with("data") {
                let rel_path = entry_path.strip_prefix("data")
                    .map_err(|e| AqjError::Archive(e.to_string()))?;
                
                if rel_path.as_os_str().is_empty() {
                    continue;
                }

                let dest_path = target_path.join(rel_path);

                if entry.header().entry_type() == EntryType::Directory {
                    fs::create_dir_all(&dest_path)?;
                } else {
                    if let Some(parent) = dest_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    entry.unpack(&dest_path)?;
                }
            }
        }

        Ok((meta, pkg_files))
    }
}

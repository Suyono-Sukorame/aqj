use thiserror::Error;
use std::path::PathBuf;

#[derive(Error, Debug)]
pub enum AqjError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Package archive error: {0}")]
    Archive(String),

    #[error("Package metadata missing: {0}")]
    MetadataMissing(String),

    #[error("Package '{0}' not found in database")]
    PackageNotFound(String),

    #[error("Package '{0}' is already installed")]
    PackageAlreadyInstalled(String),

    #[error("Checksum mismatch for '{file}': expected {expected}, got {actual}")]
    ChecksumMismatch {
        file: String,
        expected: String,
        actual: String,
    },

    #[error("File conflict: '{file}' already exists and belongs to package '{owner}'")]
    FileConflict {
        file: String,
        owner: String,
    },

    #[error("Database error at path '{path}': {message}")]
    DatabaseError {
        path: PathBuf,
        message: String,
    },

    #[error("Build error: {0}")]
    BuildError(String),

    #[error("Unsatisfied dependency: package '{package}' requires '{dependency}' which could not be resolved")]
    UnsatisfiedDependency {
        package: String,
        dependency: String,
    },

    #[error("Circular dependency detected: {chain}")]
    CircularDependency {
        chain: String,
    },

    #[error("Dependency version conflict: package '{package}' requires '{dependency}' with constraint '{constraint}', but found version '{found}'")]
    DependencyConflict {
        package: String,
        dependency: String,
        constraint: String,
        found: String,
    },
}

pub type Result<T> = std::result::Result<T, AqjError>;

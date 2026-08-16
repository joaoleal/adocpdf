//! Reading and writing files, inside the sandbox.

use std::fs;
use std::io;

use adocpdf_domain::error::DomainError;
use adocpdf_domain::ports::SourceStore;
use adocpdf_domain::sandbox::SandboxedPath;

/// Reads and writes through the operating system.
///
/// Every method takes a [`SandboxedPath`], so this adapter never performs the
/// confinement check itself — it cannot be reached with an unchecked path,
/// because there is no way to construct one of those without checking.
#[derive(Debug, Clone, Copy, Default)]
pub struct FilesystemSourceStore;

impl FilesystemSourceStore {
    /// Creates a store.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl SourceStore for FilesystemSourceStore {
    fn read(&self, path: &SandboxedPath) -> Result<String, DomainError> {
        fs::read_to_string(path.as_path()).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                DomainError::InputNotFound {
                    path: path.as_requested().to_owned(),
                }
            } else {
                DomainError::InputUnreadable {
                    path: path.as_requested().to_owned(),
                    reason: error.to_string(),
                }
            }
        })
    }

    fn write(&self, path: &SandboxedPath, bytes: &[u8]) -> Result<(), DomainError> {
        // A missing parent directory is reported as an unwritable output rather
        // than created. Creating directories implicitly would let a render put
        // files in places the caller never named — inside the root, but still
        // not what was asked for.
        fs::write(path.as_path(), bytes).map_err(|error| DomainError::OutputUnwritable {
            path: path.as_requested().to_owned(),
            reason: error.to_string(),
        })
    }
}

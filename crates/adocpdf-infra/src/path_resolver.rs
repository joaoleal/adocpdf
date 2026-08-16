//! Resolving paths against the real filesystem.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use adocpdf_domain::sandbox::{PathResolver, ResolutionError};

/// Resolves paths by asking the operating system.
///
/// Canonicalisation follows symbolic links and removes `.` and `..`, which is
/// what makes the domain's containment check meaningful: it compares locations,
/// and this is what turns a spelling into a location.
#[derive(Debug, Clone, Copy, Default)]
pub struct FilesystemPathResolver;

impl FilesystemPathResolver {
    /// Creates a resolver.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl PathResolver for FilesystemPathResolver {
    /// Resolves as much of the path as exists, then appends the rest.
    ///
    /// An output file usually does not exist yet, so canonicalising the whole
    /// path would fail exactly when confinement most needs checking — before
    /// the write. Walking up to the nearest existing ancestor gives a location
    /// to check against even for a file that has never been created.
    ///
    /// The unresolved remainder is appended verbatim. That is safe because the
    /// ancestor is fully resolved: no `..` can remain in the remainder without
    /// having been resolved away first, since a `..` below an existing
    /// directory would itself have resolved.
    fn resolve(&self, path: &Path) -> Result<PathBuf, ResolutionError> {
        let mut unresolved: Vec<OsString> = Vec::new();
        let mut current = path.to_path_buf();

        loop {
            match current.canonicalize() {
                Ok(mut resolved) => {
                    for component in unresolved.iter().rev() {
                        resolved.push(component);
                    }
                    return Ok(resolved);
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    let Some(name) = current.file_name().map(OsString::from) else {
                        return Err(ResolutionError::NotFound);
                    };
                    let Some(parent) = current.parent() else {
                        return Err(ResolutionError::NotFound);
                    };

                    unresolved.push(name);
                    current = if parent.as_os_str().is_empty() {
                        PathBuf::from(".")
                    } else {
                        parent.to_path_buf()
                    };
                }
                Err(error) => return Err(ResolutionError::Unreadable(error.to_string())),
            }
        }
    }

    fn is_directory(&self, path: &Path) -> bool {
        path.is_dir()
    }
}

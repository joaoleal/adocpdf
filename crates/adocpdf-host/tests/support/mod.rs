//! A throwaway directory for tests that need real files.
//!
//! Hand-rolled rather than pulled from a crate: the workspace's dependency rule
//! makes every third-party crate a deliberate entry in `architecture.toml`, and
//! thirty lines of `std` is a smaller thing to own than a dependency that
//! appears in the layer table forever.
//!
//! Each integration test file compiles as its own crate, so items used by only
//! some of them look unreachable from the others. That is inherent to sharing
//! a module across test binaries, not a sign of dead code.
#![allow(unreachable_pub, dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::{fs, process};

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A directory that deletes itself when it goes out of scope.
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Creates an empty directory with a name unique to this process.
    ///
    /// The name combines the process id with a counter rather than a random
    /// value, so it is unique without the test needing a source of randomness.
    pub fn new(label: &str) -> Self {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("adocpdf-{label}-{}-{unique}", process::id()));

        drop(fs::remove_dir_all(&path));
        fs::create_dir_all(&path).expect("can create a temporary directory");

        Self { path }
    }

    /// The directory itself.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Creates a subdirectory and returns its path.
    pub fn create_dir(&self, relative: &str) -> PathBuf {
        let path = self.path.join(relative);
        fs::create_dir_all(&path).expect("can create a subdirectory");
        path
    }

    /// Writes a file and returns its path.
    pub fn write(&self, relative: &str, contents: &str) -> PathBuf {
        let path = self.path.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("can create the parent directory");
        }
        fs::write(&path, contents).expect("can write the file");
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.path));
    }
}

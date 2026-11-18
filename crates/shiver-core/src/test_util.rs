//! Helpers for building packs folders in tests.

use std::path::{Path, PathBuf};

use tempfile::TempDir;

/// A temporary packs folder, deleted when dropped.
pub struct Packs(TempDir);

impl Packs {
    pub fn new() -> Self {
        Packs(TempDir::new().expect("create temp dir"))
    }

    pub fn dir(&self) -> &Path {
        self.0.path()
    }

    /// Writes `<dir>/<folder>/pack.toml` and returns its path.
    pub fn add(&self, folder: &str, pack_toml: &str) -> PathBuf {
        let folder = self.dir().join(folder);
        std::fs::create_dir_all(&folder).expect("create pack folder");
        let file = folder.join("pack.toml");
        std::fs::write(&file, pack_toml).expect("write pack.toml");
        file
    }
}

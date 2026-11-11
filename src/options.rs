use std::path::{Path, PathBuf};

/// Execution options and security boundaries for applying patches.
#[derive(Debug, Clone)]
pub struct PatchOptions {
    /// The root workspace directory to resolve relative file paths against.
    pub root_dir: PathBuf,
    /// Optional directory to store timestamped backups of modified or deleted files.
    pub backup_dir: Option<PathBuf>,
    /// Whether to automatically create backups before applying destructive edits or deletions.
    pub enable_backup: bool,
    /// Whitelisted external directories that are safe to modify outside the root.
    pub allowed_external_dirs: Vec<PathBuf>,
}

impl Default for PatchOptions {
    fn default() -> Self {
        Self {
            root_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            backup_dir: None,
            enable_backup: true,
            allowed_external_dirs: Vec::new(),
        }
    }
}

impl PatchOptions {
    pub fn new(root_dir: impl AsRef<Path>) -> Self {
        Self {
            root_dir: root_dir.as_ref().to_path_buf(),
            ..Default::default()
        }
    }

    pub fn with_root_dir(mut self, root_dir: impl AsRef<Path>) -> Self {
        self.root_dir = root_dir.as_ref().to_path_buf();
        self
    }

    pub fn with_backup_dir(mut self, backup_dir: impl AsRef<Path>) -> Self {
        self.backup_dir = Some(backup_dir.as_ref().to_path_buf());
        self
    }

    pub fn with_backup_enabled(mut self, enabled: bool) -> Self {
        self.enable_backup = enabled;
        self
    }

    pub fn with_allowed_external_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.allowed_external_dirs.push(dir.as_ref().to_path_buf());
        self
    }
}

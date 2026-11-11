use std::path::{Component, Path, PathBuf};

/// Normalizes a path by removing redundant `.` and resolving `..` components lexically.
pub fn normalize_path(path: &Path) -> PathBuf {
    let components = path.components();
    let mut normalized = PathBuf::new();

    for component in components {
        match component {
            Component::Prefix(..) => {
                normalized.push(component.as_os_str());
            }
            Component::RootDir => {
                normalized.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(c) => {
                normalized.push(c);
            }
        }
    }
    normalized
}

/// Verifies whether a given path stays strictly within the root directory or allowed external directories.
pub fn is_safe_path(name: &str, root: &Path, allowed_external_dirs: &[PathBuf]) -> bool {
    let clean_name = name.replace('\\', "/");
    let path = Path::new(&clean_name);

    let abs_target = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };

    let normalized = normalize_path(&abs_target);
    let norm_root = normalize_path(root);

    if normalized.starts_with(&norm_root) {
        return true;
    }

    for ext in allowed_external_dirs {
        let norm_ext = normalize_path(ext);
        if normalized.starts_with(&norm_ext) {
            return true;
        }
    }

    false
}

/// Creates a timestamped backup of the given file before modification or deletion.
pub fn backup_file(root: &Path, file_path: &Path, relative_filename: &str, backup_dir: Option<&Path>) {
    if !file_path.exists() || !file_path.is_file() {
        return;
    }

    let default_backup = root.join(".patchcraft").join("backups");
    let base_dir = backup_dir.unwrap_or(&default_backup);

    let safe_rel = relative_filename.replace('\\', "/");
    let target_path = base_dir.join(&safe_rel);

    if let Some(parent) = target_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let date_str = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let file_name_with_timestamp = if let Some(ext) = target_path.extension() {
        if let Some(name) = target_path.file_stem() {
            format!(
                "{}_{}.{}",
                name.to_string_lossy(),
                date_str,
                ext.to_string_lossy()
            )
        } else {
            format!(
                "{}_{}",
                target_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
                date_str
            )
        }
    } else {
        format!(
            "{}_{}",
            target_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy(),
            date_str
        )
    };

    let final_target_path = target_path.with_file_name(file_name_with_timestamp);
    let _ = std::fs::copy(file_path, &final_target_path);
}

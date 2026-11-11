use std::path::Path;

use crate::fs::{backup_file, is_safe_path};
use crate::matcher::fuzzy_replace_search;
use crate::models::{FileObject, PatchActionKind, PatchActionResult};
use crate::options::PatchOptions;
use crate::parser::clean_code_fence;

/// Applies all file actions to disk atomically according to the specified options.
pub fn apply_patches(options: &PatchOptions, file_obj: &FileObject) -> Vec<PatchActionResult> {
    let mut results = Vec::new();
    let root = &options.root_dir;

    for patch in &file_obj.patch {
        let file_path = &patch.path;
        if file_path.is_empty() || !is_safe_path(file_path, root, &options.allowed_external_dirs) {
            results.push(PatchActionResult {
                path: file_path.clone(),
                kind: PatchActionKind::SkippedUnsafe(file_path.clone()),
            });
            continue;
        }

        let resolved_path = root.join(file_path);
        let content = clean_code_fence(&patch.content);
        let search = clean_code_fence(&patch.search);
        let rename_to = &patch.rename_to;
        let anchor = &patch.anchor;

        if options.enable_backup {
            backup_file(root, &resolved_path, file_path, options.backup_dir.as_deref());
        }

        match patch.mode.as_str() {
            "create" => {
                if let Some(parent) = resolved_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                match std::fs::write(&resolved_path, &content) {
                    Ok(_) => results.push(PatchActionResult {
                        path: file_path.clone(),
                        kind: PatchActionKind::Created(file_path.clone()),
                    }),
                    Err(e) => results.push(PatchActionResult {
                        path: file_path.clone(),
                        kind: PatchActionKind::Failed {
                            path: file_path.clone(),
                            reason: e.to_string(),
                        },
                    }),
                }
            }
            "edit" => {
                if let Ok(file_text) = std::fs::read_to_string(&resolved_path) {
                    let updated_content = if !search.is_empty() && file_text.contains(&search) {
                        file_text.replace(&search, &content)
                    } else if !search.is_empty() {
                        let file_norm = file_text.replace("\r\n", "\n");
                        let search_norm = search.replace("\r\n", "\n");
                        let content_norm = content.replace("\r\n", "\n");
                        if file_norm.contains(&search_norm) {
                            file_norm.replace(&search_norm, &content_norm)
                        } else if !search.trim().is_empty() && file_text.contains(search.trim()) {
                            file_text.replace(search.trim(), content.trim())
                        } else if let Some(fuzzy_replaced) = fuzzy_replace_search(&file_text, &search, &content) {
                            fuzzy_replaced
                        } else {
                            results.push(PatchActionResult {
                                path: file_path.clone(),
                                kind: PatchActionKind::Failed {
                                    path: file_path.clone(),
                                    reason: "search target not found in file".to_string(),
                                },
                            });
                            continue;
                        }
                    } else {
                        content.clone()
                    };

                    if updated_content != file_text {
                        match std::fs::write(&resolved_path, updated_content) {
                            Ok(_) => results.push(PatchActionResult {
                                path: file_path.clone(),
                                kind: PatchActionKind::Edited(file_path.clone()),
                            }),
                            Err(e) => results.push(PatchActionResult {
                                path: file_path.clone(),
                                kind: PatchActionKind::Failed {
                                    path: file_path.clone(),
                                    reason: e.to_string(),
                                },
                            }),
                        }
                    } else {
                        results.push(PatchActionResult {
                            path: file_path.clone(),
                            kind: PatchActionKind::Edited(file_path.clone()),
                        });
                    }
                } else {
                    // Self-healing: if file does not exist to edit, create it with content
                    if let Some(parent) = resolved_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    match std::fs::write(&resolved_path, &content) {
                        Ok(_) => results.push(PatchActionResult {
                            path: file_path.clone(),
                            kind: PatchActionKind::Created(file_path.clone()),
                        }),
                        Err(e) => results.push(PatchActionResult {
                            path: file_path.clone(),
                            kind: PatchActionKind::Failed {
                                path: file_path.clone(),
                                reason: format!("file did not exist to edit, and fallback creation failed: {}", e),
                            },
                        }),
                    }
                }
            }
            "rename" if !rename_to.is_empty() && is_safe_path(rename_to, root, &options.allowed_external_dirs) => {
                let resolved_rename_to = root.join(rename_to);
                if let Some(parent) = resolved_rename_to.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                match std::fs::rename(&resolved_path, &resolved_rename_to) {
                    Ok(_) => results.push(PatchActionResult {
                        path: file_path.clone(),
                        kind: PatchActionKind::Renamed {
                            from: file_path.clone(),
                            to: rename_to.clone(),
                        },
                    }),
                    Err(e) => results.push(PatchActionResult {
                        path: file_path.clone(),
                        kind: PatchActionKind::Failed {
                            path: file_path.clone(),
                            reason: e.to_string(),
                        },
                    }),
                }
            }
            "delete" => {
                if resolved_path.exists() {
                    match std::fs::remove_file(&resolved_path) {
                        Ok(_) => results.push(PatchActionResult {
                            path: file_path.clone(),
                            kind: PatchActionKind::Deleted(file_path.clone()),
                        }),
                        Err(e) => results.push(PatchActionResult {
                            path: file_path.clone(),
                            kind: PatchActionKind::Failed {
                                path: file_path.clone(),
                                reason: e.to_string(),
                            },
                        }),
                    }
                } else {
                    results.push(PatchActionResult {
                        path: file_path.clone(),
                        kind: PatchActionKind::Deleted(file_path.clone()),
                    });
                }
            }
            "append" => {
                if let Ok(file_text) = std::fs::read_to_string(&resolved_path) {
                    let target_search = if !search.is_empty() && file_text.contains(&search) {
                        search.as_str()
                    } else if !search.trim().is_empty() && file_text.contains(search.trim()) {
                        search.trim()
                    } else {
                        ""
                    };

                    let updated_content = if !target_search.is_empty() {
                        if anchor == "up" {
                            let insertion = format!("{}\n{}", content, target_search);
                            file_text.replace(target_search, &insertion)
                        } else {
                            let insertion = format!("{}\n{}", target_search, content);
                            file_text.replace(target_search, &insertion)
                        }
                    } else {
                        if anchor == "up" {
                            format!("{}\n{}", content, file_text)
                        } else {
                            format!("{}\n{}", file_text, content)
                        }
                    };

                    match std::fs::write(&resolved_path, updated_content) {
                        Ok(_) => results.push(PatchActionResult {
                            path: file_path.clone(),
                            kind: PatchActionKind::Appended {
                                path: file_path.clone(),
                                anchor: anchor.clone(),
                            },
                        }),
                        Err(e) => results.push(PatchActionResult {
                            path: file_path.clone(),
                            kind: PatchActionKind::Failed {
                                path: file_path.clone(),
                                reason: e.to_string(),
                            },
                        }),
                    }
                } else {
                    if let Some(parent) = resolved_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    match std::fs::write(&resolved_path, &content) {
                        Ok(_) => results.push(PatchActionResult {
                            path: file_path.clone(),
                            kind: PatchActionKind::Created(file_path.clone()),
                        }),
                        Err(e) => results.push(PatchActionResult {
                            path: file_path.clone(),
                            kind: PatchActionKind::Failed {
                                path: file_path.clone(),
                                reason: e.to_string(),
                            },
                        }),
                    }
                }
            }
            _ => {}
        }
    }

    results
}

/// Applies all file actions to disk atomically under the specified root directory.
/// Convenience wrapper around `apply_patches`.
pub fn apply_file_actions(root: &Path, file_obj: &FileObject) -> Vec<PatchActionResult> {
    let options = PatchOptions::new(root);
    apply_patches(&options, file_obj)
}

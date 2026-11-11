use crate::models::{FileObject, PatchObj, ResponsePayload};
use crate::parser::json::extract_balanced_json;
use crate::parser::search_replace::{
    clean_code_fence, extract_file_path_from_header, extract_rename_target, parse_anchor_block,
    parse_search_replace_block,
};

/// Parses markdown file actions (File: ..., Action: ..., ```lang ... ``` blocks)
pub fn parse_markdown_patches(response: &str, session_id: &str) -> Option<FileObject> {
    let mut patches = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_mode = "create".to_string();
    let mut current_anchor = String::new();
    let mut summary = None;

    let lines: Vec<&str> = response.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Check for Summary
        if line.to_lowercase().starts_with("### summary")
            || line.to_lowercase().starts_with("**summary**:")
            || line.to_lowercase().starts_with("summary:")
        {
            let sum_text = if let Some((_, text)) = line.split_once(':') {
                text.trim().to_string()
            } else if i + 1 < lines.len() && !lines[i + 1].trim().starts_with('#') {
                i += 1;
                lines[i].trim().to_string()
            } else {
                String::new()
            };
            if !sum_text.is_empty() {
                summary = Some(sum_text);
            }
        }

        // Check for File header
        if line.starts_with('#') || line.starts_with("**File") || line.starts_with("File:") {
            if let Some(path) = extract_file_path_from_header(line) {
                current_path = Some(path);
                current_mode = "create".to_string();
                current_anchor = String::new();
            }
        }

        // Check for Action line
        if line.to_lowercase().contains("action") {
            let lower = line.to_lowercase();
            if lower.contains("create") {
                current_mode = "create".to_string();
            } else if lower.contains("edit") {
                current_mode = "edit".to_string();
            } else if lower.contains("append") {
                current_mode = "append".to_string();
                if lower.contains("anchor: `up`") || lower.contains("anchor: up") {
                    current_anchor = "up".to_string();
                } else {
                    current_anchor = "down".to_string();
                }
            } else if lower.contains("delete") {
                current_mode = "delete".to_string();
                if let Some(ref path) = current_path {
                    patches.push(PatchObj {
                        path: path.clone(),
                        mode: "delete".to_string(),
                        anchor: String::new(),
                        search: String::new(),
                        content: String::new(),
                        rename_to: String::new(),
                    });
                    current_path = None;
                }
            } else if lower.contains("rename") {
                current_mode = "rename".to_string();
                if let Some(target) = extract_rename_target(line) {
                    if let Some(ref path) = current_path {
                        patches.push(PatchObj {
                            path: path.clone(),
                            mode: "rename".to_string(),
                            anchor: String::new(),
                            search: String::new(),
                            content: String::new(),
                            rename_to: target,
                        });
                        current_path = None;
                    }
                }
            }
        }

        // Check for Code fence
        if line.starts_with("```") {
            let mut block_lines = Vec::new();
            i += 1;
            while i < lines.len() && !lines[i].trim().starts_with("```") {
                block_lines.push(lines[i]);
                i += 1;
            }
            let block = block_lines.join("\n");

            if let Some(ref path) = current_path {
                let has_diff = block.contains("<<<<<<< SEARCH")
                    || (block.lines().any(|l| l.starts_with("- ") || l.starts_with("--"))
                        && block.lines().any(|l| l.starts_with("+ ") || l.starts_with("++")));
                let effective_mode = if current_mode == "create" && has_diff {
                    "edit"
                } else {
                    current_mode.as_str()
                };

                match effective_mode {
                    "create" => {
                        patches.push(PatchObj {
                            path: path.clone(),
                            mode: "create".to_string(),
                            anchor: String::new(),
                            search: String::new(),
                            content: block,
                            rename_to: String::new(),
                        });
                    }
                    "edit" => {
                        let (search, replace) = parse_search_replace_block(&block);
                        patches.push(PatchObj {
                            path: path.clone(),
                            mode: "edit".to_string(),
                            anchor: String::new(),
                            search,
                            content: replace,
                            rename_to: String::new(),
                        });
                    }
                    "append" => {
                        let (anchor_code, insert_code) = parse_anchor_block(&block);
                        let anchor_dir = if current_anchor.is_empty() {
                            "down".to_string()
                        } else {
                            current_anchor.clone()
                        };
                        patches.push(PatchObj {
                            path: path.clone(),
                            mode: "append".to_string(),
                            anchor: anchor_dir,
                            search: anchor_code,
                            content: insert_code,
                            rename_to: String::new(),
                        });
                    }
                    _ => {}
                }
                current_path = None;
                current_mode = "create".to_string();
                current_anchor = String::new();
            }
        }

        i += 1;
    }

    if !patches.is_empty() {
        Some(FileObject {
            session_id: session_id.to_string(),
            summary,
            patch: patches,
        })
    } else {
        None
    }
}

/// Convert the model's response to a `FileObject` and ensure the `session_id` is set.
/// Tries markdown parsing first, then JSON payloads, then stripped markdown fences, and balanced JSON extraction.
pub fn file_object(response: &str, session_id: &str) -> Option<FileObject> {
    // 1. First attempt: Parse structured Markdown (primary format for coding LLMs)
    if let Some(file_obj) = parse_markdown_patches(response, session_id) {
        if !file_obj.patch.is_empty() {
            return Some(file_obj);
        }
    }

    // 2. Second attempt: Direct JSON parsing (backward compatibility)
    if let Ok(payload) = serde_json::from_str::<ResponsePayload>(response) {
        return finalize(payload, session_id);
    }

    // 3. Third attempt: Strip outer code fence (```json ... ``` or ``` ... ```) and parse JSON
    let stripped = clean_code_fence(response);
    if let Ok(payload) = serde_json::from_str::<ResponsePayload>(&stripped) {
        return finalize(payload, session_id);
    }

    // 4. Fourth attempt: Extract balanced JSON from within text or code markers
    if let Some(json_str) = extract_balanced_json(response) {
        if let Ok(payload) = serde_json::from_str::<ResponsePayload>(&json_str) {
            return finalize(payload, session_id);
        }
    }
    if let Some(json_str) = extract_balanced_json(&stripped) {
        if let Ok(payload) = serde_json::from_str::<ResponsePayload>(&json_str) {
            return finalize(payload, session_id);
        }
    }

    None
}

/// Finalizes the file object by setting the session ID and cleaning code fences from patches
fn finalize(payload: ResponsePayload, session_id: &str) -> Option<FileObject> {
    let mut file_obj = payload.file;
    if file_obj.session_id.is_empty() {
        file_obj.session_id = session_id.to_string();
    }
    for patch in &mut file_obj.patch {
        patch.content = clean_code_fence(&patch.content);
        patch.search = clean_code_fence(&patch.search);
    }
    Some(file_obj)
}

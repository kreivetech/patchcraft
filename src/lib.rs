//! # patchcraft
//!
//! Fast, resilient, and surgical code patch parser and atomic disk applier designed for LLM coding workflows.
//!
//! ## Overview
//!
//! Large Language Models (LLMs) generate edits in various formats: structured markdown code fences,
//! unified diffs, search/replace blocks (`<<<<<<< SEARCH ... ======= ... >>>>>>> REPLACE`), or JSON payloads.
//! Furthermore, LLMs frequently introduce subtle whitespace, indentation, or newline variations that break
//! traditional patch tools like `git apply` or `patch`.
//!
//! `patchcraft` handles these challenges by providing:
//! - **Multi-format Parsing**: Extracts patches from Markdown (`### File: ...`, `**Action**: ...`), search/replace blocks, unified diffs, or JSON payloads.
//! - **Fuzzy Line & Indentation Matching**: Tolerates indentation and blank-line discrepancies to locate target search blocks reliably.
//! - **Atomic & Safe Application**: Prevents path-traversal attacks (`../`) outside designated project roots, creates automated timestamped backups, and executes atomic file updates.
//! - **Self-healing Fallbacks**: Gracefully creates files if an edit is requested for a file that does not yet exist.
//! - **Chat Proposal Sanitizer**: Converts raw machine diff instructions into clean, human-readable markdown summaries for chat interfaces.

pub mod applier;
pub mod error;
pub mod fs;
pub mod matcher;
pub mod models;
pub mod options;
pub mod parser;

pub use applier::{apply_file_actions, apply_patches};
pub use error::PatchError;
pub use fs::{backup_file, is_safe_path, normalize_path};
pub use matcher::fuzzy_replace_search;
pub use models::{FileObject, PatchActionKind, PatchActionResult, PatchObj, ResponsePayload};
pub use options::PatchOptions;
pub use parser::{
    clean_chat_proposal, clean_code_fence, convert_search_replace_to_unified_diff,
    extract_balanced_json, extract_file_path_from_header, extract_rename_target, file_object,
    parse_anchor_block, parse_markdown_patches, parse_search_replace_block,
};

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_clean_code_fence() {
        assert_eq!(clean_code_fence("```rust\nfn main() {}\n```"), "fn main() {}");
        assert_eq!(clean_code_fence("`foo.rs`"), "foo.rs");
        assert_eq!(clean_code_fence("plain text"), "plain text");
    }

    #[test]
    fn test_parse_markdown_patches_create_and_edit() {
        let text = r#"
### Summary: Added user model and updated routes

### File: src/models/user.rs
**Action**: `create`

```rust
pub struct User {
    pub id: u64,
}
```

### File: src/routes.rs
**Action**: `edit`

```rust
<<<<<<< SEARCH
// routes
=======
// new routes
>>>>>>> REPLACE
```
"#;
        let file_obj = parse_markdown_patches(text, "test_sess").expect("Should parse patches");
        assert_eq!(file_obj.patch.len(), 2);
        assert_eq!(file_obj.patch[0].path, "src/models/user.rs");
        assert_eq!(file_obj.patch[0].mode, "create");
        assert_eq!(file_obj.patch[1].path, "src/routes.rs");
        assert_eq!(file_obj.patch[1].mode, "edit");
        assert_eq!(file_obj.patch[1].search, "// routes");
        assert_eq!(file_obj.patch[1].content, "// new routes");
    }

    #[test]
    fn test_apply_file_actions_create_edit_delete() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let file_obj = FileObject {
            session_id: "test".to_string(),
            summary: Some("test actions".to_string()),
            patch: vec![
                PatchObj {
                    path: "hello.txt".to_string(),
                    mode: "create".to_string(),
                    anchor: String::new(),
                    search: String::new(),
                    content: "hello world".to_string(),
                    rename_to: String::new(),
                },
            ],
        };

        let results = apply_file_actions(root, &file_obj);
        assert_eq!(results.len(), 1);
        assert_eq!(std::fs::read_to_string(root.join("hello.txt")).unwrap(), "hello world");

        // Edit
        let edit_obj = FileObject {
            session_id: "test".to_string(),
            summary: None,
            patch: vec![
                PatchObj {
                    path: "hello.txt".to_string(),
                    mode: "edit".to_string(),
                    anchor: String::new(),
                    search: "world".to_string(),
                    content: "rust".to_string(),
                    rename_to: String::new(),
                },
            ],
        };
        let edit_results = apply_file_actions(root, &edit_obj);
        assert_eq!(edit_results.len(), 1);
        assert_eq!(std::fs::read_to_string(root.join("hello.txt")).unwrap(), "hello rust");

        // Delete
        let delete_obj = FileObject {
            session_id: "test".to_string(),
            summary: None,
            patch: vec![
                PatchObj {
                    path: "hello.txt".to_string(),
                    mode: "delete".to_string(),
                    anchor: String::new(),
                    search: String::new(),
                    content: String::new(),
                    rename_to: String::new(),
                },
            ],
        };
        let del_results = apply_file_actions(root, &delete_obj);
        assert_eq!(del_results.len(), 1);
        assert!(!root.join("hello.txt").exists());
    }

    #[test]
    fn test_fuzzy_replace_search_indentation_variance() {
        let original = "fn calculate() {\n    let a = 10;\n    let b = 20;\n    a + b\n}";
        // Search has 2-space indentation instead of 4
        let search = "  let a = 10;\n  let b = 20;";
        let replacement = "    let a = 100;\n    let b = 200;";

        let result = fuzzy_replace_search(original, search, replacement).expect("Should match fuzzy");
        assert!(result.contains("let a = 100;"));
        assert!(result.contains("let b = 200;"));
        assert!(!result.contains("let a = 10;"));
    }

    #[test]
    fn test_apply_file_actions_self_healing_missing_edit() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Target file does not exist yet
        let edit_missing = FileObject {
            session_id: "test".to_string(),
            summary: None,
            patch: vec![PatchObj {
                path: "brand_new.rs".to_string(),
                mode: "edit".to_string(),
                anchor: String::new(),
                search: "some search".to_string(),
                content: "pub fn hello() {}".to_string(),
                rename_to: String::new(),
            }],
        };

        let results = apply_file_actions(root, &edit_missing);
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0].kind, PatchActionKind::Created(_)));
        assert_eq!(
            std::fs::read_to_string(root.join("brand_new.rs")).unwrap(),
            "pub fn hello() {}"
        );
    }

    #[test]
    fn test_clean_chat_proposal_formatting() {
        let raw = r#"Here is the proposal:

### File: src/main.rs
Action: edit
```rust
<<<<<<< SEARCH
    println!("old");
=======
    println!("new");
>>>>>>> REPLACE
```

All set!"#;

        let cleaned = clean_chat_proposal(raw);
        assert!(!cleaned.contains("<<<<<<< SEARCH"));
        assert!(!cleaned.contains(">>>>>>> REPLACE"));
        assert!(!cleaned.contains("Action: edit"));
        assert!(cleaned.contains("#### 📄 `src/main.rs`"));
        assert!(cleaned.contains("```diff"));
        assert!(cleaned.contains("-     println!(\"old\");"));
        assert!(cleaned.contains("+     println!(\"new\");"));
        assert!(cleaned.contains("All set!"));
    }

    #[test]
    fn test_path_traversal_protection() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let malicious_patch = FileObject {
            session_id: "sec_test".to_string(),
            summary: None,
            patch: vec![PatchObj {
                path: "../../etc/passwd".to_string(),
                mode: "create".to_string(),
                anchor: String::new(),
                search: String::new(),
                content: "malicious".to_string(),
                rename_to: String::new(),
            }],
        };

        let results = apply_file_actions(root, &malicious_patch);
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0].kind, PatchActionKind::SkippedUnsafe(_)));
    }

    #[test]
    fn test_json_payload_extraction() {
        let json_text = r#"Some intro text
{
  "file": {
    "session_id": "json_sess",
    "summary": "Updated file via json",
    "patch": [
      {
        "path": "src/lib.rs",
        "mode": "create",
        "content": "// json content"
      }
    ]
  }
}
Some outro text"#;

        let parsed = file_object(json_text, "default_sess").expect("Should extract balanced JSON");
        assert_eq!(parsed.session_id, "json_sess");
        assert_eq!(parsed.patch.len(), 1);
        assert_eq!(parsed.patch[0].path, "src/lib.rs");
        assert_eq!(parsed.patch[0].content, "// json content");
    }
}

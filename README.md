# patchcraft 🪄

[![Crates.io](https://img.shields.io/crates/v/patchcraft.svg)](https://crates.io/crates/patchcraft)
[![Documentation](https://docs.rs/patchcraft/badge.svg)](https://docs.rs/patchcraft)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

**Fast, resilient, and surgical code patch parser and atomic disk applier designed for LLM coding workflows.**

---

## 🌟 Why Patchcraft?

Large Language Models (LLMs) rarely output identical, rigid git diffs. In the real world, coding agents face several challenges:
1. **Multi-format Variance**: LLMs generate edits across various formats — structured Markdown blocks, search/replace blocks (`<<<<<<< SEARCH ... ======= ... >>>>>>> REPLACE`), unified diffs, or structured JSON payloads.
2. **Indentation & Whitespace Drift**: Minor tab-vs-space discrepancies or newline mismatches cause standard tools like `git apply` or `patch` to fail catastrophically.
3. **Security Vulnerabilities**: Naive patch appliers are vulnerable to path traversal attacks (e.g. `../../etc/shadow`).
4. **Chat Ergonomics**: Raw patch instructions (with markers and delimiters) are messy to present directly to end users.

`patchcraft` solves all of these issues in a lightweight, zero-bloat, memory-safe Rust library.

---

## 🚀 Features

- **Multi-Format Parsing**:
  - Markdown action headers (`### File: path/to/file`, `**Action**: create|edit|append|rename|delete`).
  - Search/Replace blocks (`<<<<<<< SEARCH`, `=======`, `>>>>>>> REPLACE`).
  - Unified diffs (`-`, `+`, `--`, `++`).
  - Anchor-based inserts (`<<<<<<< ANCHOR ... ======= ... >>>>>>> REPLACE`).
  - Direct or balanced JSON payloads embedded within conversational text.
- **Fuzzy Indentation Matching**:
  - Resilient anchor search that matches lines even when LLMs alter indentation levels or blank lines.
- **Atomic & Safe Execution**:
  - Strict path traversal checks preventing changes outside the designated root workspace.
  - Automated timestamped file backups before destructive edits or deletions.
  - Self-healing fallback: if an `edit` target does not yet exist, it creates the file cleanly.
- **Chat Proposal Sanitizer**:
  - Converts raw machine instructions into formatted, clean markdown proposals with color-coded diff syntax highlighting (````diff`).

---

## 📦 Installation

Add `patchcraft` to your `Cargo.toml`:

```toml
[dependencies]
patchcraft = "0.1.0"
```

Or via `cargo`:

```bash
cargo add patchcraft
```

---

## 🛠️ Quickstart

### 1. Parsing and Applying Patches from LLM Output

```rust
use std::path::Path;
use patchcraft::{file_object, apply_patches, PatchOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let llm_response = r#"
Sure! Here is the change:

### File: src/main.rs
**Action**: `edit`

```rust
<<<<<<< SEARCH
    println!("Hello, old world!");
=======
    println!("Hello, Patchcraft!");
>>>>>>> REPLACE
```
"#;

    // Parse patches from Markdown or JSON
    let file_obj = file_object(llm_response, "session-123")
        .expect("Failed to parse patches from response");

    // Configure options (custom root dir, backups, allowed external paths)
    let options = PatchOptions::new("./my_project")
        .with_backup_enabled(true);

    // Apply patches safely to disk
    let results = apply_patches(&options, &file_obj);

    for result in results {
        println!("File: {} -> Status: {:?}", result.path, result.kind);
    }

    Ok(())
}
```

### 2. Sanitizing LLM Output for User-Facing Chat

Transform machine diff markers into clean markdown diffs ready for rendering in a chat UI:

```rust
use patchcraft::clean_chat_proposal;

let raw_llm_output = r#"
### File: src/lib.rs
Action: edit
```rust
<<<<<<< SEARCH
fn old() {}
=======
fn new() {}
>>>>>>> REPLACE
```
"#;

let formatted_proposal = clean_chat_proposal(raw_llm_output);
println!("{}", formatted_proposal);
// Outputs clean:
// #### 📄 `src/lib.rs`
// ```diff
// - fn old() {}
// + fn new() {}
// ```
```

---

## 🛡️ Security

`patchcraft` rigorously validates every relative path against the configured `root_dir`. Any attempt to escape the root boundary (e.g. `../../etc/passwd` or symbolic path traversals) is detected, rejected, and returned as `PatchActionKind::SkippedUnsafe`.

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).

/// Cleans markdown code fences (```lang ... ``` or inline `...`) from raw content or search strings.
pub fn clean_code_fence(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.starts_with("```") {
        let after_opening = if let Some(idx) = trimmed.find('\n') {
            &trimmed[idx + 1..]
        } else {
            let without_ticks = trimmed.trim_start_matches('`');
            return without_ticks.trim_end_matches('`').to_string();
        };

        let trimmed_end = after_opening.trim_end();
        if let Some(without_closing) = trimmed_end.strip_suffix("```") {
            without_closing
                .trim_end_matches('\r')
                .trim_end_matches('\n')
                .to_string()
        } else {
            after_opening.to_string()
        }
    } else if trimmed.starts_with('`')
        && trimmed.ends_with('`')
        && trimmed.len() >= 2
        && !trimmed[1..trimmed.len() - 1].contains('`')
    {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

pub fn extract_file_path_from_header(line: &str) -> Option<String> {
    for word in line.split_whitespace() {
        let clean = word.trim_matches(|c| {
            c == '`' || c == '*' || c == ':' || c == '#' || c == '"' || c == '\'' || c == '(' || c == ')'
        });
        if (clean.contains('/') || clean.contains('.')) && !clean.contains("../") && !clean.contains("..\\") {
            return Some(clean.to_string());
        }
    }
    None
}

pub fn extract_rename_target(line: &str) -> Option<String> {
    if let Some((_, target)) = line.split_once("->") {
        let clean = target
            .trim()
            .trim_matches(|c| c == '`' || c == '"' || c == '\'' || c == '*' || c == ' ');
        if !clean.is_empty() && !clean.contains("../") && !clean.contains("..\\") {
            return Some(clean.to_string());
        }
    }
    None
}

pub fn parse_search_replace_block(block: &str) -> (String, String) {
    if block.contains("<<<<<<< SEARCH") && block.contains("=======") && block.contains(">>>>>>> REPLACE") {
        let search_part = block.split("<<<<<<< SEARCH").nth(1).unwrap_or("");
        let (search, rest) = search_part.split_once("=======").unwrap_or(("", ""));
        let replace = rest.split(">>>>>>> REPLACE").next().unwrap_or("");
        return (
            search.trim_matches('\n').trim_matches('\r').to_string(),
            replace.trim_matches('\n').trim_matches('\r').to_string(),
        );
    }

    // Fallback: unified diff syntax (- / + or -- / ++)
    let lines: Vec<&str> = block.lines().collect();
    let has_minus = lines.iter().any(|l| l.starts_with("- ") || l.starts_with("--"));
    let has_plus = lines.iter().any(|l| l.starts_with("+ ") || l.starts_with("++"));

    if has_minus || has_plus {
        let mut search_lines = Vec::new();
        let mut replace_lines = Vec::new();

        for line in lines {
            if let Some(stripped) = line.strip_prefix("--") {
                search_lines.push(stripped.trim_start());
            } else if let Some(stripped) = line.strip_prefix("++") {
                replace_lines.push(stripped.trim_start());
            } else if let Some(stripped) = line.strip_prefix("- ") {
                search_lines.push(stripped);
            } else if let Some(stripped) = line.strip_prefix("+ ") {
                replace_lines.push(stripped);
            } else if let Some(stripped) = line.strip_prefix("  ") {
                search_lines.push(stripped);
                replace_lines.push(stripped);
            } else {
                search_lines.push(line);
                replace_lines.push(line);
            }
        }

        return (search_lines.join("\n"), replace_lines.join("\n"));
    }

    (String::new(), block.to_string())
}

pub fn parse_anchor_block(block: &str) -> (String, String) {
    if block.contains("<<<<<<< ANCHOR") && block.contains("=======") && block.contains(">>>>>>> REPLACE") {
        let anchor_part = block.split("<<<<<<< ANCHOR").nth(1).unwrap_or("");
        let (anchor_code, rest) = anchor_part.split_once("=======").unwrap_or(("", ""));
        let insert_code = rest.split(">>>>>>> REPLACE").next().unwrap_or("");
        return (
            anchor_code.trim_matches('\n').trim_matches('\r').to_string(),
            insert_code.trim_matches('\n').trim_matches('\r').to_string(),
        );
    }

    (String::new(), block.to_string())
}

/// Converts search/replace block into a clean unified diff string (- and + lines)
pub fn convert_search_replace_to_unified_diff(search: &str, replace: &str) -> String {
    let search_lines: Vec<&str> = search.lines().collect();
    let replace_lines: Vec<&str> = replace.lines().collect();

    let mut prefix_len = 0;
    while prefix_len < search_lines.len()
        && prefix_len < replace_lines.len()
        && search_lines[prefix_len] == replace_lines[prefix_len]
    {
        prefix_len += 1;
    }

    let mut suffix_len = 0;
    while suffix_len < (search_lines.len() - prefix_len)
        && suffix_len < (replace_lines.len() - prefix_len)
        && search_lines[search_lines.len() - 1 - suffix_len] == replace_lines[replace_lines.len() - 1 - suffix_len]
    {
        suffix_len += 1;
    }

    let search_mid = &search_lines[prefix_len..search_lines.len() - suffix_len];
    let replace_mid = &replace_lines[prefix_len..replace_lines.len() - suffix_len];

    let mut diff = Vec::new();

    let ctx_pre_start = prefix_len.saturating_sub(2);
    for line in &search_lines[ctx_pre_start..prefix_len] {
        diff.push(format!("  {}", line));
    }

    for line in search_mid {
        diff.push(format!("- {}", line));
    }
    for line in replace_mid {
        diff.push(format!("+ {}", line));
    }

    let suf_start = search_lines.len() - suffix_len;
    let suf_end = (suf_start + 2).min(search_lines.len());
    for line in &search_lines[suf_start..suf_end] {
        diff.push(format!("  {}", line));
    }

    diff.join("\n")
}

/// Cleans raw machine patch instructions (SEARCH/REPLACE markers, Action headers) from AI response
/// to present a clean human-readable conversation proposal in chat.
pub fn clean_chat_proposal(response: &str) -> String {
    let lines: Vec<&str> = response.lines().collect();
    let mut cleaned_lines = Vec::new();
    let mut i = 0;

    let mut current_file: Option<String> = None;
    let mut current_action = String::new();

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Check for File header
        if trimmed.starts_with('#') || trimmed.starts_with("**File") || trimmed.starts_with("File:") {
            if let Some(path) = extract_file_path_from_header(trimmed) {
                current_file = Some(path);
                current_action = "create".to_string();
                i += 1;
                continue;
            }
        }

        // Check for Action line
        if trimmed.to_lowercase().starts_with("action:") || trimmed.to_lowercase().starts_with("**action**:") {
            let lower = trimmed.to_lowercase();
            if lower.contains("create") {
                current_action = "create".to_string();
            } else if lower.contains("edit") {
                current_action = "edit".to_string();
            } else if lower.contains("delete") {
                current_action = "delete".to_string();
                if let Some(ref path) = current_file {
                    cleaned_lines.push(format!("#### 🗑️ `{}` *(Deleted)*", path));
                    current_file = None;
                }
            } else if lower.contains("rename") {
                current_action = "rename".to_string();
                if let Some(target) = extract_rename_target(trimmed) {
                    if let Some(ref path) = current_file {
                        cleaned_lines.push(format!("#### 🔄 `{}` → `{}`", path, target));
                        current_file = None;
                    }
                }
            }
            i += 1;
            continue;
        }

        // Check for code fence
        if trimmed.starts_with("```") {
            let lang = trimmed.trim_start_matches('`').trim();
            let mut block_lines = Vec::new();
            i += 1;
            while i < lines.len() && !lines[i].trim().starts_with("```") {
                block_lines.push(lines[i]);
                i += 1;
            }
            let block = block_lines.join("\n");

            if block.contains("<<<<<<< SEARCH") && block.contains("=======") && block.contains(">>>>>>> REPLACE") {
                let (search, replace) = parse_search_replace_block(&block);
                let diff_content = convert_search_replace_to_unified_diff(&search, &replace);

                if let Some(ref path) = current_file {
                    cleaned_lines.push(format!("#### 📄 `{}`", path));
                    current_file = None;
                }
                cleaned_lines.push("```diff".to_string());
                cleaned_lines.push(diff_content);
                cleaned_lines.push("```".to_string());
            } else if let Some(ref path) = current_file {
                if current_action == "create" {
                    cleaned_lines.push(format!("#### 📄 `{}` *(Novo arquivo)*", path));
                } else {
                    cleaned_lines.push(format!("#### 📄 `{}`", path));
                }
                let fence_lang = if lang.is_empty() { "text" } else { lang };
                cleaned_lines.push(format!("```{}", fence_lang));
                cleaned_lines.push(block);
                cleaned_lines.push("```".to_string());
                current_file = None;
            } else {
                let fence_lang = if lang.is_empty() { "" } else { lang };
                cleaned_lines.push(format!("```{}", fence_lang));
                cleaned_lines.push(block);
                cleaned_lines.push("```".to_string());
            }

            i += 1;
            continue;
        }

        cleaned_lines.push(line.to_string());
        i += 1;
    }

    cleaned_lines.join("\n")
}

/// Searches for target content in a file using normalized line-by-line fuzzy matching
/// to withstand indentation, whitespace, and newline discrepancies.
pub fn fuzzy_replace_search(file_text: &str, search: &str, content: &str) -> Option<String> {
    if search.trim().is_empty() {
        return None;
    }

    let file_norm = file_text.replace("\r\n", "\n");
    let search_norm = search.replace("\r\n", "\n");
    let content_norm = content.replace("\r\n", "\n");

    let file_lines: Vec<&str> = file_norm.lines().collect();
    let search_lines: Vec<&str> = search_norm.lines().collect();

    // 1. Trimmed line-by-line slice matching
    let trimmed_search: Vec<&str> = search_lines.iter().map(|l| l.trim()).collect();
    let first_non_empty = trimmed_search.iter().position(|l| !l.is_empty())?;
    let last_non_empty = trimmed_search.iter().rposition(|l| !l.is_empty())?;
    let core_search = &trimmed_search[first_non_empty..=last_non_empty];

    if core_search.is_empty() {
        return None;
    }

    let search_len = core_search.len();
    if file_lines.len() >= search_len {
        for i in 0..=(file_lines.len() - search_len) {
            let mut matches = true;
            for k in 0..search_len {
                if file_lines[i + k].trim() != core_search[k] {
                    matches = false;
                    break;
                }
            }

            if matches {
                let start_idx = i;
                let end_idx = i + search_len;

                let mut result_lines = Vec::new();
                result_lines.extend_from_slice(&file_lines[..start_idx]);
                result_lines.push(&content_norm);
                result_lines.extend_from_slice(&file_lines[end_idx..]);

                return Some(result_lines.join("\n"));
            }
        }
    }

    // 2. Non-empty line sequence matching (ignoring blank line disparities)
    let non_empty_search: Vec<&str> = search_lines
        .iter()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    if !non_empty_search.is_empty() {
        let file_non_empty_indices: Vec<(usize, &str)> = file_lines
            .iter()
            .enumerate()
            .map(|(idx, l)| (idx, l.trim()))
            .filter(|(_, l)| !l.is_empty())
            .collect();

        if file_non_empty_indices.len() >= non_empty_search.len() {
            let nes_len = non_empty_search.len();
            for i in 0..=(file_non_empty_indices.len() - nes_len) {
                let mut matches = true;
                for k in 0..nes_len {
                    if file_non_empty_indices[i + k].1 != non_empty_search[k] {
                        matches = false;
                        break;
                    }
                }

                if matches {
                    let start_line = file_non_empty_indices[i].0;
                    let end_line = file_non_empty_indices[i + nes_len - 1].0 + 1;

                    let mut result_lines = Vec::new();
                    result_lines.extend_from_slice(&file_lines[..start_line]);
                    result_lines.push(&content_norm);
                    result_lines.extend_from_slice(&file_lines[end_line..]);

                    return Some(result_lines.join("\n"));
                }
            }
        }
    }

    None
}

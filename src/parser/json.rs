/// Extracts the first balanced JSON object `{ ... }` from a string, correctly
/// handling string boundaries, escapes, and nested braces.
pub fn extract_balanced_json(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let bytes = text.as_bytes();

    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;

    for (i, &b) in bytes[start..].iter().enumerate() {
        let c = b as char;

        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }

        match c {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(text[start..start + i + 1].to_string());
                }
            }
            _ => {}
        }
    }

    None
}

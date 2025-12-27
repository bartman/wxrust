

pub fn side_by_side_diff(old: &str, new: &str) -> String {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let mut result = String::new();

    // Simple line-by-line diff
    let max_len = old_lines.len().max(new_lines.len());
    for i in 0..max_len {
        let old_line = old_lines.get(i).map_or("", |v| v);
        let new_line = new_lines.get(i).map_or("", |v| v);

        if old_line != new_line {
            if !old_line.is_empty() {
                result.push_str(&format!("- {}\n", old_line));
            }
            if !new_line.is_empty() {
                result.push_str(&format!("+ {}\n", new_line));
            }
        } else if !old_line.is_empty() {
            result.push_str(&format!("  {}\n", old_line));
        }
    }

    if result.is_empty() {
        "No differences found.".to_string()
    } else {
        format!("Differences:\n{}", result)
    }
}
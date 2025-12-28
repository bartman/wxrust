use crate::models::{JDay, EBlock, ExerciseWrapper, Exercise, Set};

#[allow(dead_code)]
pub fn parse_workout(text: &str) -> Result<JDay, String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;

    // Skip date line
    if i >= lines.len() {
        return Err("No date line".to_string());
    }
    i += 1;

    // Parse bw line
    if i >= lines.len() {
        return Err("No bw line".to_string());
    }
    let bw_line = lines[i];
    if !bw_line.starts_with("@ ") {
        return Err("Expected @ bw line".to_string());
    }
    let bw_str = &bw_line[2..].trim();
    if !bw_str.ends_with(" bw") {
        return Err("Expected 'bw' at end".to_string());
    }
    let bw_val: f32 = bw_str[..bw_str.len()-3].trim().parse().map_err(|_| "Invalid bw number")?;
    i += 1;

    // Now parse the rest, building log and extracting exercises
    let mut exercises = Vec::new();
    let mut eblocks = Vec::new();
    let mut log_lines = Vec::new();

    while i < lines.len() {
        let line = lines[i];
        if line.trim().is_empty() {
            log_lines.push(line.to_string());
            i += 1;
            continue;
        }
        if line.starts_with("//") {
            log_lines.push(line.to_string());
            i += 1;
            continue;
        }
        if line.starts_with('#') {
            // Parse exercise name
            let name = line[1..].trim().to_string();
            let eid = name.clone();

            exercises.push(ExerciseWrapper {
                exercise: Exercise {
                    id: eid.clone(),
                    name: name.clone(),
                    ex_type: None,
                }
            });

            log_lines.push(format!("EBLOCK:{}", eid));

            // Parse sets
            i += 1;
            let mut sets = Vec::new();
            while i < lines.len() && !lines[i].starts_with('#') && !lines[i].starts_with("//") && !lines[i].trim().is_empty() {
                let line = lines[i];
                let set_line = line.trim();
                match parse_set_line(set_line) {
                    Ok(set) => {
                        sets.extend(set);
                    }
                    Err(_) => {
                        // Not a set line, add to log
                        log_lines.push(line.to_string());
                    }
                }
                i += 1;
            }

            eblocks.push(EBlock {
                eid,
                sets,
            });
        } else {
            log_lines.push(line.to_string());
            i += 1;
        }
    }

    // Build log
    let mut log = log_lines.join("\n");
    if !log_lines.is_empty() {
        log += "\n";
    }

    Ok(JDay {
        log,
        bw: Some(bw_val),
        eblocks,
        exercises,
    })
}

// examples:
//   405               - ( Set { w=405, r=1, s=1, c="" } )
//   405 cccc          - ( Set { w=405, r=1, s=1, c="cccc" } )
//   405 x 2           - ( Set { w=405, r=2, s=1, c="" } )
//   405 x 2 x 3       - ( Set { w=405, r=2, s=3, c="" } )
//   405 x 2, 3        - ( Set { w=405, r=2, s=1, c="" }, Set { w=405, r=3, s=1, c="" } )
//   405 x 2, 3 cccc   - ( Set { w=405, r=2, s=1, c="" }, Set { w=405, r=3, s=1, c="cccc" } )
//   405, 406 x 2      - ( Set { w=405, r=2, s=1, c="" }, Set { w=406, r=2, s=1, c="" } )
//   405, 406 x 2 cccc - ( Set { w=405, r=2, s=1, c="" }, Set { w=406, r=2, s=1, c="cccc" } )

pub fn is_weight_part(s: &str) -> bool {
    s.chars().all(|c| c.is_digit(10) || c == ',' || c == '.')
}

pub fn parse_weights(s: &str) -> Result<Vec<f32>, String> {
    s.split(',')
        .map(|p| p.trim().parse::<f32>().map_err(|_| format!("Invalid weight: {}", p)))
        .collect()
}

pub fn parse_reps_and_comment(s: &str) -> Result<(Vec<u32>, Option<String>), String> {
    let parts: Vec<&str> = s.split(',').map(|p| p.trim()).collect();
    let mut reps = Vec::new();
    let mut comment = None;
    for (i, part) in parts.iter().enumerate() {
        let sub_parts: Vec<&str> = part.split_whitespace().collect();
        if sub_parts.is_empty() {
            return Err("Empty rep".to_string());
        }
        let num_str = sub_parts[0];
        let num: u32 = num_str.parse().map_err(|_| format!("Invalid rep: {}", num_str))?;
        reps.push(num);
        let rest = sub_parts[1..].join(" ");
        if !rest.is_empty() {
            if i == parts.len() - 1 {
                comment = Some(rest);
            } else {
                return Err("Comment only allowed on last rep".to_string());
            }
        }
    }
    Ok((reps, comment))
}

pub fn parse_set_line(line: &str) -> Result<Vec<Set>, String> {
    let line = line.trim();
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Empty line".to_string());
    }
    // Collect weights parts until not weight part
    let mut weights_end = 0;
    while weights_end < parts.len() && is_weight_part(parts[weights_end]) {
        weights_end += 1;
    }
    let weights_str = parts[0..weights_end].join(" ");
    let weights = parse_weights(&weights_str)?;
    if weights.is_empty() {
        return Err("No weights".to_string());
    }
    let mut comment = None;
    let mut sets = 1;
    let mut reps = vec![1];
    if weights_end < parts.len() && parts[weights_end] == "x" {
        // Has 'x'
        let reps_start = weights_end + 1;
        let mut sets_start = None;
        let mut i = reps_start;
        while i < parts.len() {
            if parts[i] == "x" {
                sets_start = Some(i);
                break;
            }
            i += 1;
        }
        let reps_str = parts[reps_start..sets_start.unwrap_or(parts.len())].join(" ");
        let (parsed_reps, reps_comment) = parse_reps_and_comment(&reps_str)?;
        reps = parsed_reps;
        comment = reps_comment;
        if let Some(ss) = sets_start {
            let sets_i = ss + 1;
            if sets_i >= parts.len() {
                return Err("Missing sets after x".to_string());
            }
            sets = parts[sets_i].parse().map_err(|_| "Invalid sets")?;
            let comment_i = sets_i + 1;
            if comment_i < parts.len() {
                comment = Some(parts[comment_i..].join(" "));
            }
        }
    } else {
        // No 'x', comment is the rest
        if weights_end < parts.len() {
            comment = Some(parts[weights_end..].join(" "));
        }
    }
    // Now create the sets
    let mut result = Vec::new();
    for &w in &weights {
        for &r in &reps {
            result.push(Set {
                w: Some(w),
                r: Some(r),
                s: Some(sets),
                lb: Some(0.0),
                rpe: None,
                c: None,
                set_type: Some(0),
            });
        }
    }
    // Set comment on the last set
    if let Some(c) = comment {
        if let Some(last) = result.last_mut() {
            last.c = Some(c);
        }
    }
    Ok(result)
}

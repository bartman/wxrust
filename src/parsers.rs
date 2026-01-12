use crate::models::{JDay, EBlock, ExerciseWrapper, Exercise, Set};
use regex::Regex;
use lazy_static::lazy_static;

pub const LBS_PER_KG: f32 = 2.20462;

lazy_static! {
    static ref BW_REGEX: Regex = Regex::new(r"^@ *([1-9][0-9]*\.?[0-9]*) *(kg|lbs)? *bw$").unwrap();
}

#[derive(Clone)]
pub struct ParserOptions {
    /// If true, weights without explicit units are interpreted as kg.
    /// If false, weights without explicit units are interpreted as lbs.
    pub user_wants_kg: bool,
}

impl ParserOptions {
    pub fn new(user_wants_kg: bool) -> Self {
        Self { user_wants_kg }
    }

    pub fn default() -> Self {
        Self { user_wants_kg: true }
    }
}

pub fn parse_bw_line(lines: &[&str], i: &mut usize) -> Result<Option<f32>, String> {
    // Skip empty lines
    while *i < lines.len() && lines[*i].trim().is_empty() {
        *i += 1;
    }
    if *i >= lines.len() {
        return Ok(None);
    }
    let line = lines[*i];
    if let Some(caps) = BW_REGEX.captures(line.trim()) {
        let mut lb_to_kg = false;
        if let Some(unit_match) = caps.get(2) {
            if unit_match.as_str() == "lbs" {
                lb_to_kg = true;
            }
        };
        if let Some(num_match) = caps.get(1) {
            let bw_val: f32 = num_match.as_str().parse().map_err(|_| format!("Line {}: Invalid bw number: {}", *i + 1, num_match.as_str()))?;
            let bw = if lb_to_kg {
                Some(bw_val / LBS_PER_KG)
            } else {
                Some(bw_val)
            };
            *i += 1;
            Ok(bw)
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

#[allow(dead_code)]
pub fn parse_workout(text: &str) -> Result<JDay, String> {
    parse_workout_with_options(text, &ParserOptions::default())
}

#[allow(dead_code)]
pub fn parse_workout_with_options(text: &str, options: &ParserOptions) -> Result<JDay, String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;

    // Skip date line
    if i >= lines.len() {
        return Err("Line 1: No date line".to_string());
    }
    i += 1;

    // Parse bw line (optional)
    let bw = parse_bw_line(&lines, &mut i)?;

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

            if name.is_empty() {
                // Lone #, treat as comment
                log_lines.push(line.to_string());
                i += 1;
                continue;
            }

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
            let mut failed_lines = Vec::new();
            while i < lines.len() && !lines[i].starts_with('#') && !lines[i].starts_with("//") && !lines[i].trim().is_empty() {
                let line = lines[i];
                let set_line = line.trim();
                match parse_set_line_with_options(set_line, options) {
                    Ok(set) => {
                        sets.extend(set);
                    }
                    Err(_e) => {
                        // Not a set line, collect for later
                        failed_lines.push(line.to_string());
                    }
                }
                i += 1;
            }

            if sets.is_empty() {
                // No valid sets, treat the # line and failed lines as comments
                log_lines.pop(); // remove the EBLOCK line
                log_lines.push(line.to_string());
                log_lines.extend(failed_lines);
                // Don't add the exercise and eblock
                exercises.pop(); // remove the added exercise
            } else {
                // Valid sets found, add failed lines to log
                log_lines.extend(failed_lines);
                eblocks.push(EBlock {
                    eid,
                    sets,
                });
            }
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
        bw,
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
    s.chars().all(|c| c.is_digit(10) || c == ',' || c == '.' || c == '+' || c == '-') || s.to_lowercase().starts_with("bw") || s.to_lowercase().contains("kg") || s.to_lowercase().contains("lb") || s.to_lowercase().contains("lbs")
}

pub fn parse_weight(s: &str) -> Result<(f32, bool, i32), String> {
    let s = s.trim();
    let lower = s.to_lowercase();
    if lower.starts_with("bw") {
        let mut usebw = 1;
        let mut v = 0.0;
        let rest = &s[2..].trim();
        if rest.starts_with('+') {
            usebw = 1;
            let num = rest[1..].trim();
            if !num.is_empty() {
                v = num.parse().map_err(|_| "Invalid BW+ weight")?;
            }
        } else if rest.starts_with('-') {
            usebw = -1;
            let num = rest[1..].trim();
            if !num.is_empty() {
                v = num.parse().map_err(|_| "Invalid BW- weight")?;
            }
        } else if !rest.is_empty() {
            return Err("Invalid BW syntax".to_string());
        }
        Ok((v, false, usebw))
    } else {
        let mut lb = false;
        let num_end = s.find(|c: char| !c.is_digit(10) && c != '.');
        let num_str = if let Some(end) = num_end { &s[..end] } else { s };
        let unit = if let Some(end) = num_end { &s[end..].trim().to_lowercase() } else { "" };
        if unit == "lb" || unit == "lbs" {
            lb = true;
        } else if unit == "kg" {
            lb = false;
        } else if !unit.is_empty() {
            return Err(format!("Invalid unit: {}", unit));
        }
        let v: f32 = num_str.parse().map_err(|_| format!("Invalid weight: {}", num_str))?;
        Ok((v, lb, 0))
    }
}

pub fn parse_weights(s: &str) -> Result<Vec<(f32, bool, i32)>, String> {
    s.split(',')
        .map(|p| parse_weight(p.trim()))
        .collect()
}

pub fn parse_reps_and_comment(s: &str) -> Result<(Vec<u32>, Option<String>), String> {
    let s = s.trim();
    let mut reps = Vec::new();
    let mut i = 0;
    let chars: Vec<char> = s.chars().collect();
    while i < chars.len() {
        // Skip whitespace
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }
        // Parse number
        let start = i;
        while i < chars.len() && chars[i].is_digit(10) {
            i += 1;
        }
        if start == i {
            // No number, the rest is comment
            break;
        }
        let num_str: String = chars[start..i].iter().collect();
        let num: u32 = num_str.parse().map_err(|_| format!("Invalid rep: {}", num_str))?;
        reps.push(num);
        // Skip whitespace
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        // Check for ,
        if i < chars.len() && chars[i] == ',' {
            i += 1;
            // Continue to next number
        } else {
            // No more , , the rest is comment
            break;
        }
    }
    // The rest from i is comment
    let mut comment_start = i;
    while comment_start < chars.len() && chars[comment_start].is_whitespace() {
        comment_start += 1;
    }
    let comment = if comment_start < chars.len() {
        Some(chars[comment_start..].iter().collect())
    } else {
        None
    };
    Ok((reps, comment))
}

#[allow(dead_code)]
pub fn parse_rpe(s: &str) -> Option<f32> {
    let s = s.trim();
    if !s.starts_with('@') {
        return None;
    }
    let s = &s[1..].trim();
    // Optional "rpe "
    let s = if s.to_lowercase().starts_with("rpe ") {
        &s[4..].trim()
    } else {
        s
    };
    // Parse number, can be float
    s.parse().ok()
}

#[allow(dead_code)]
pub fn parse_set_line(line: &str) -> Result<Vec<Set>, String> {
    parse_set_line_with_options(line, &ParserOptions::default())
}

pub fn parse_set_line_with_options(line: &str, options: &ParserOptions) -> Result<Vec<Set>, String> {
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
    // Parse RPE from comment
    let mut rpe = None;
    let mut final_comment = comment.clone();
    if let Some(c) = &comment {
        if c.trim().starts_with('@') {
            let trimmed = c.trim();
            let after_at = &trimmed[1..];
            let rpe_part_end = after_at.find(' ').unwrap_or(after_at.len());
            let rpe_part = &after_at[..rpe_part_end];
            if let Ok(r) = rpe_part.parse::<f32>() {
                rpe = Some(r);
                let rpe_full = &trimmed[..1 + rpe_part_end];
                let rest = trimmed.strip_prefix(rpe_full).unwrap_or(trimmed).trim();
                final_comment = if rest.is_empty() { None } else { Some(rest.to_string()) };
            }
        }
    }
    // Determine lb if any weights have lbs units
    // When user_wants_kg is false, weights without explicit units are in lbs
    let store_w_in_lbs = !options.user_wants_kg;
    let mut show_lb = 0.0;
    for (_, parsed_lb, _) in &weights {
        if *parsed_lb {
            show_lb = 1.0;
        }
    }
    // Now create the sets
    let mut result = Vec::new();
    for (w, parsed_lb, usebw) in weights {
        let mut w_in_kg = w;
        // Convert to kg if needed:
        // - If weight has explicit "lbs" unit, convert from lbs to kg
        // - If weight has no explicit unit but user wants lbs (store_w_in_lbs=true), convert from lbs to kg
        if parsed_lb && !store_w_in_lbs {
            w_in_kg = w / LBS_PER_KG
        } else if !parsed_lb && store_w_in_lbs {
            w_in_kg = w / LBS_PER_KG
        }
        for &r in &reps {
            result.push(Set {
                w: Some(w_in_kg),
                r: Some(r),
                s: Some(sets),
                lb: Some(show_lb),
                rpe,
                c: final_comment.clone(),
                set_type: Some(0),
                usebw: if usebw != 0 { Some(usebw) } else { None },
            });
        }
    }
    // If multiple sets, only last has comment
    if result.len() > 1 {
        for i in 0..result.len() - 1 {
            result[i].c = None;
        }
    }
    Ok(result)
}

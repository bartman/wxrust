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

fn parse_set_line(line: &str) -> Result<Vec<Set>, String> {
    let set_parts = line.trim();

    // Check for compressed weights
    if set_parts.contains(',') {
        // Like "405, 445 x 3"
        let x_pos = set_parts.find(" x ").ok_or("Missing ' x ' in compressed set")?;
        let weights_str = &set_parts[..x_pos];
        let rest = &set_parts[x_pos+3..];

        let weights: Vec<f32> = weights_str.split(',')
            .map(|s| s.trim().parse().map_err(|_| format!("Invalid weight: {}", s)))
            .collect::<Result<Vec<_>, _>>()?;

        let (reps, sets, comment) = parse_reps_and_sets(rest)?;

        let mut result = Vec::new();
        for w in weights {
            result.push(Set {
                w: Some(w),
                r: reps,
                s: sets,
                lb: Some(0.0), // assume kg
                rpe: None,
                c: comment.clone(),
                set_type: Some(0),
            });
        }
        Ok(result)
    } else if set_parts.contains(" x ") {
        // Like "135 x 10" or "135 x 10 x 3"
        let parts: Vec<&str> = set_parts.split(" x ").collect();
        if parts.len() < 2 {
            return Err(format!("Invalid set format: {}", set_parts));
        }
        let weight: f32 = parts[0].trim().parse().map_err(|_| "Invalid weight")?;
        let rest = parts[1..].join(" x ");
        let (reps, sets, comment) = parse_reps_and_sets(&rest)?;

        Ok(vec![Set {
            w: Some(weight),
            r: reps,
            s: sets,
            lb: Some(0.0), // assume kg
            rpe: None,
            c: comment,
            set_type: Some(0),
        }])
    } else {
        Err(format!("Unrecognized set format: {}", set_parts))
    }
}

fn parse_reps_and_sets(rest: &str) -> Result<(Option<u32>, Option<u32>, Option<String>), String> {
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.is_empty() {
        return Err("No reps after x".to_string());
    }

    let reps: u32 = parts[0].parse().map_err(|_| "Invalid reps")?;
    let mut sets = Some(1);
    let mut comment = None;

    if parts.len() > 1 {
        if parts[1] == "x" {
            if parts.len() < 3 {
                return Err("Missing sets after x".to_string());
            }
            sets = Some(parts[2].parse().map_err(|_| "Invalid sets")?);
            if parts.len() > 3 {
                comment = Some(parts[3..].join(" "));
            }
        } else {
            comment = Some(parts[1..].join(" "));
        }
    }

    Ok((Some(reps), sets, comment))
}

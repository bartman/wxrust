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

    // Collect program notes until first #exercise
    let mut program_notes = Vec::new();
    while i < lines.len() && !lines[i].starts_with('#') {
        if !lines[i].trim().is_empty() {
            program_notes.push(lines[i].trim());
        }
        i += 1;
    }

    // Now parse exercises
    let mut exercises = Vec::new();
    let mut eblocks = Vec::new();

    while i < lines.len() {
        let line = lines[i].trim();
        if line.is_empty() {
            i += 1;
            continue;
        }
        if line.starts_with("//") {
            // URL/comment, add to program notes
            program_notes.push(line);
            i += 1;
            continue;
        }
        if !line.starts_with('#') {
            return Err(format!("Expected exercise line starting with #, got: {}", line));
        }

        // Parse exercise name
        let exercise_line = &line[1..]; // remove #
        let parts: Vec<&str> = exercise_line.split('#').map(|s| s.trim()).collect();
        let name = parts[0].to_string();
        let ex_type = if parts.len() > 1 { Some(parts[1].to_string()) } else { None };

        let eid = name.clone();

        exercises.push(ExerciseWrapper {
            exercise: Exercise {
                id: eid.clone(),
                name: name.clone(),
                ex_type,
            }
        });

        // Parse sets
        i += 1;
        let mut sets = Vec::new();
        while i < lines.len() && !lines[i].starts_with('#') && !lines[i].starts_with("//") && !lines[i].trim().is_empty() {
            let set_line = lines[i].trim();
            let set = parse_set_line(set_line)?;
            sets.extend(set);
            i += 1;
        }

        eblocks.push(EBlock {
            eid,
            sets,
        });
    }

    // Build log
    let mut log = program_notes.join("\n");
    if !log.is_empty() && !eblocks.is_empty() {
        log += "\n";
    }
    for eblock in eblocks.iter() {
        log += &format!("EBLOCK:{}\n", eblock.eid);
    }

    Ok(JDay {
        log,
        bw: Some(bw_val),
        eblocks,
        exercises,
    })
}

fn parse_set_line(line: &str) -> Result<Vec<Set>, String> {
    // Split by spaces, but handle comments at the end
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        return Ok(vec![]);
    }

    // Check if last part is a comment (not a number)
    let (set_parts, comment) = if parts.len() > 3 && !parts.last().unwrap().chars().all(|c| c.is_numeric() || c == '.' || c == 'x') {
        let comment_start = line.rfind(' ').unwrap();
        (&line[..comment_start], Some(line[comment_start..].trim().to_string()))
    } else {
        (line, None)
    };

    let set_parts = set_parts.trim();

    // Check for compressed weights
    if set_parts.contains(',') {
        // Like "405, 445 x 3"
        let x_pos = set_parts.find(" x ").ok_or("Missing ' x ' in compressed set")?;
        let weights_str = &set_parts[..x_pos];
        let rest = &set_parts[x_pos+3..];

        let weights: Vec<f32> = weights_str.split(',')
            .map(|s| s.trim().parse().map_err(|_| format!("Invalid weight: {}", s)))
            .collect::<Result<Vec<_>, _>>()?;

        let (reps, sets) = parse_reps_and_sets(rest)?;

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
        if parts.len() < 2 || parts.len() > 3 {
            return Err(format!("Invalid set format: {}", set_parts));
        }
        let weight: f32 = parts[0].trim().parse().map_err(|_| "Invalid weight")?;
        let (reps, mut sets) = parse_reps_and_sets(parts[1])?;
        if parts.len() == 3 {
            sets = Some(parts[2].trim().parse().map_err(|_| "Invalid sets")?);
        }

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

fn parse_reps_and_sets(rest: &str) -> Result<(Option<u32>, Option<u32>), String> {
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.is_empty() {
        return Err("No reps after x".to_string());
    }

    let reps: u32 = parts[0].parse().map_err(|_| "Invalid reps")?;
    let sets = if parts.len() > 1 {
        Some(parts[1].parse().map_err(|_| "Invalid sets")?)
    } else {
        Some(1)
    };

    Ok((Some(reps), sets))
}

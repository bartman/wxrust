use std::collections::HashMap;
use regex::Regex;

use crate::models::{JDay, Set, Exercise};

pub fn format_weight(w: f32, lb: bool) -> String {
    if lb {
        format!("{:.0}", w * 2.20462)
    } else {
        format!("{:.0}", w)
    }
}

pub fn compress_sets(sets: &[Set]) -> Vec<String> {
    let mut groups: Vec<(f32, Vec<u32>)> = Vec::new();
    for set in sets {
        let weight = set.w.unwrap_or(0.0);
        let reps = set.r.unwrap_or(0);
        if let Some(pos) = groups.iter().position(|(w, _)| *w == weight) {
            groups[pos].1.push(reps);
        } else {
            groups.push((weight, vec![reps]));
        }
    }
    groups.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let mut result = Vec::new();
    for (weight, reps) in groups {
        let w_str = format_weight(weight, true);
        if reps.len() == 1 {
            result.push(format!("{} x {}", w_str, reps[0]));
        } else {
            let r_str = reps.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(", ");
            result.push(format!("{} x {}", w_str, r_str));
        }
    }
    result
}

pub fn format_eblocks(jday: &JDay) -> String {
    let mut ex_map: HashMap<String, &Exercise> = HashMap::new();
    for ex_wrap in &jday.exercises {
        ex_map.insert(ex_wrap.exercise.id.clone(), &ex_wrap.exercise);
    }
    let mut lines = Vec::new();
    for eblock in &jday.eblocks {
        if let Some(ex) = ex_map.get(&eblock.eid) {
            lines.push("#".to_string() + &ex.name);
            lines.extend(compress_sets(&eblock.sets));
        }
    }
    lines.join("\n")
}

pub fn format_workout(jday: &JDay) -> String {
    let formatted_eblocks = format_eblocks(jday);
    let re = Regex::new(r"EBLOCK:\d+").unwrap();
    re.replace_all(&jday.log, &("\n".to_string() + &formatted_eblocks + "\n")).to_string()
}
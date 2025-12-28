use std::collections::HashMap;
use lazy_static::lazy_static;
use ansi_term::Colour;
use atty;

//use crate::models::{JDay, Set, Exercise, EBlock, User};
use crate::models::{JDay, Set, Exercise, EBlock};

lazy_static! {
    static ref COLOR_ENABLED: bool = {
        let color_arg = std::env::var("WXRUST_COLOR").unwrap_or("auto".to_string());
        match color_arg.as_str() {
            "always" => true,
            "never" => false,
            "auto" => atty::is(atty::Stream::Stdout),
            _ => atty::is(atty::Stream::Stdout),
        }
    };

    pub static ref STDERR_COLOR_ENABLED: bool = {
        let color_arg = std::env::var("WXRUST_COLOR").unwrap_or("auto".to_string());
        match color_arg.as_str() {
            "always" => true,
            "never" => false,
            "auto" => atty::is(atty::Stream::Stderr),
            _ => atty::is(atty::Stream::Stderr),
        }
    };
}

pub fn color_date(s: &str) -> String {
    color_date_internal(s, *COLOR_ENABLED)
}

fn color_date_internal(s: &str, color_enabled: bool) -> String {
    if color_enabled {
        Colour::RGB(157, 78, 221).paint(s).to_string()
    } else {
        s.to_string()
    }
}

#[allow(dead_code)]
pub fn color_bw(s: &str) -> String {
    color_bw_internal(s, *COLOR_ENABLED)
}

fn color_bw_internal(s: &str, color_enabled: bool) -> String {
    if color_enabled {
        Colour::RGB(58, 134, 255).paint(s).to_string()
    } else {
        s.to_string()
    }
}

fn color_exercise_internal(s: &str, color_enabled: bool) -> String {
    if color_enabled {
        Colour::RGB(0, 150, 255).paint(s).to_string()
    } else {
        s.to_string()
    }
}

fn color_weight_internal(s: &str, color_enabled: bool) -> String {
    if color_enabled {
        Colour::RGB(255, 121, 0).paint(s).to_string()
    } else {
        s.to_string()
    }
}

fn color_reps_internal(s: &str, color_enabled: bool) -> String {
    if color_enabled {
        Colour::RGB(0, 187, 249).paint(s).to_string()
    } else {
        s.to_string()
    }
}

fn color_sets_internal(s: &str, color_enabled: bool) -> String {
    if color_enabled {
        Colour::RGB(241, 91, 181).paint(s).to_string()
    } else {
        s.to_string()
    }
}



pub fn format_weight(w: f32, lb: bool) -> String {
    if lb {
        format!("{:.0}", w * 2.20462)
    } else {
        format!("{:.0}", w)
    }
}

#[allow(dead_code)]
pub fn format_set(set: &Set) -> String {
    format_set_internal(set, *COLOR_ENABLED)
}

#[allow(dead_code)]
fn format_set_internal(set: &Set, color_enabled: bool) -> String {
    let w = set.w.unwrap_or(0.0);
    let r = set.r.unwrap_or(0);
    let s = set.s.unwrap_or(1);
    let rpe = set.rpe.unwrap_or(0.0);
    let lb = set.lb.unwrap_or(0.0) == 1.0;
    let w_str = color_weight_internal(&format_weight(w, lb), color_enabled);
    let mut line = w_str;
    if r > 0 {
        line += " x ";
        line += &color_reps_internal(&r.to_string(), color_enabled);
    }
    if s > 1 {
        line += " x ";
        line += &color_sets_internal(&s.to_string(), color_enabled);
    }
    if rpe > 0.0 {
        line += &format!(" @{}", rpe);
    }
    if let Some(c) = &set.c {
        if !c.is_empty() {
            line += &format!(" {}", c);
        }
    }
    line
}

#[allow(dead_code)]
pub fn compress_sets(sets: &[Set]) -> Vec<String> {
    compress_sets_internal(sets, *COLOR_ENABLED)
}

fn compress_sets_internal(sets: &[Set], color_enabled: bool) -> Vec<String> {
    let mut compressed = Vec::new();
    let mut i = 0;
    while i < sets.len() {
        let set = &sets[i];
        if set.set_type.unwrap_or(0) != 0 {
            compressed.push(format_set_internal(set, color_enabled));
            i += 1;
            continue;
        }
        let w = set.w.unwrap_or(0.0);
        let r = set.r.unwrap_or(0);
        let _s = set.s.unwrap_or(1);
        let rpe = set.rpe.unwrap_or(0.0);
        let lb = set.lb.unwrap_or(0.0) == 1.0;
        // check for same weight consecutive
        let mut same_weight = vec![r];
        let mut j = i + 1;
        while j < sets.len() {
            let next = &sets[j];
            if next.set_type.unwrap_or(0) != 0 || next.w != set.w || next.rpe != set.rpe || next.lb != set.lb || next.s != set.s {
                break;
            }
            same_weight.push(next.r.unwrap_or(0));
            j += 1;
        }
        if same_weight.len() > 1 {
            let w_str = color_weight_internal(&format_weight(w, lb), color_enabled);
            let r_str = same_weight.iter().map(|&r| color_reps_internal(&r.to_string(), color_enabled)).collect::<Vec<_>>().join(", ");
            let mut line = format!("{} x {}", w_str, r_str);
            if rpe > 0.0 {
                line += &format!(" @{}", rpe);
            }
            compressed.push(line);
            i = j;
        } else {
            // check for same rep
            let mut same_rep = vec![w];
            let mut j = i + 1;
            while j < sets.len() {
                let next = &sets[j];
                if next.set_type.unwrap_or(0) != 0 || next.r != set.r || next.rpe != set.rpe || next.lb != set.lb || next.s != set.s {
                    break;
                }
                same_rep.push(next.w.unwrap_or(0.0));
                j += 1;
            }
            if same_rep.len() > 1 {
                let w_str = same_rep.iter().map(|&w| color_weight_internal(&format_weight(w, lb), color_enabled)).collect::<Vec<_>>().join(", ");
                let r_str = color_reps_internal(&r.to_string(), color_enabled);
                let mut line = format!("{} x {}", w_str, r_str);
                if rpe > 0.0 {
                    line += &format!(" @{}", rpe);
                }
                compressed.push(line);
                i = j;
            } else {
                compressed.push(format_set_internal(set, color_enabled));
                i += 1;
            }
        }
    }
    compressed
}

#[allow(dead_code)]
pub fn format_single_eblock(jday: &JDay, eblock: &EBlock) -> String {
    format_single_eblock_internal(jday, eblock, *COLOR_ENABLED)
}

fn format_single_eblock_internal(jday: &JDay, eblock: &EBlock, color_enabled: bool) -> String {
    let mut ex_map: HashMap<String, &Exercise> = HashMap::new();
    for ex_wrap in &jday.exercises {
        ex_map.insert(ex_wrap.exercise.id.clone(), &ex_wrap.exercise);
    }
    let mut lines = Vec::new();
    if let Some(ex) = ex_map.get(&eblock.eid) {
        lines.push("#".to_string() + &color_exercise_internal(&ex.name, color_enabled));
        lines.extend(compress_sets_internal(&eblock.sets, color_enabled));
    }
    lines.join("\n")
}

pub fn summarize_workout(jday: &JDay) -> String {
    summarize_workout_internal(jday, *COLOR_ENABLED)
}

fn summarize_workout_internal(jday: &JDay, color_enabled: bool) -> String {
    let mut ex_map: HashMap<String, &Exercise> = HashMap::new();
    for ex_wrap in &jday.exercises {
        ex_map.insert(ex_wrap.exercise.id.clone(), &ex_wrap.exercise);
    }
    let mut summaries = Vec::new();
    for eblock in &jday.eblocks {
        if let Some(ex) = ex_map.get(&eblock.eid) {
            // Find the heaviest set: max weight, then max reps
            let mut max_weight = 0.0;
            let mut max_reps = 0;
            for set in &eblock.sets {
                let w = set.w.unwrap_or(0.0);
                let r = set.r.unwrap_or(0);
                if w > max_weight || (w == max_weight && r > max_reps) {
                    max_weight = w;
                    max_reps = r;
                }
            }
            if max_weight > 0.0 {
                let lb = eblock.sets.iter().any(|s| s.lb.unwrap_or(0.0) == 1.0);
                let w_str = color_weight_internal(&format_weight(max_weight, lb), color_enabled);
                let r_str = color_reps_internal(&max_reps.to_string(), color_enabled);
                summaries.push(format!("#{}  {}x{}", color_exercise_internal(&ex.name, color_enabled), w_str, r_str));
            }
        }
    }
    summaries.join("; ")
}

pub fn format_workout(date: &str, jday: &JDay, user_wants_kg: bool) -> String {
    format_workout_internal(date, jday, user_wants_kg, 0, *COLOR_ENABLED)
}

fn format_workout_internal(date: &str, jday: &JDay, user_wants_kg: bool, bw_precision: usize, color_enabled: bool) -> String {
    let mut result = jday.log.clone();
    for eblock in &jday.eblocks {
        let formatted = format_single_eblock_internal(jday, eblock, color_enabled);
        let placeholder = format!("EBLOCK:{}", eblock.eid);
        result = result.replace(&placeholder, &formatted);
    }
    let bw = jday.bw.unwrap_or(0.0);
    let bwtxt = if user_wants_kg {
        &format!("{:.*}", bw_precision, bw)
    } else {
        &format!("{:.*}", bw_precision, bw * 2.20462) // convert kg to lb
    };
    format!("{}\n@ {} bw\n{}", color_date_internal(date, color_enabled), color_bw_internal(bwtxt, color_enabled), result)
}

#[allow(dead_code)]
pub fn format_workout_no_color(date: &str, jday: &JDay, user_wants_kg: bool) -> String {
    format_workout_internal(date, jday, user_wants_kg, 0, false)
}

pub fn format_workout_for_cache(date: &str, jday: &JDay) -> String {
    format_workout_internal(date, jday, true, 4, false)
}
